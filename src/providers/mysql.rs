//! MySQL provider — Homebrew for installation, envswitch for service management

use crate::domain::{RemoteVersion, RunningService};
use chrono::Utc;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub struct MySqlProvider;

impl MySqlProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = std::process::Command::new("brew")
            .args(["search", "mysql"])
            .output()
            .map_err(|_| "Homebrew not found".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions = BTreeSet::new();

        for line in text.lines() {
            let line = line.trim();
            if let Some(ver) = line.strip_prefix("mysql@") {
                let ver = ver.split_whitespace().next().unwrap_or(ver);
                if ver.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    versions.insert(ver.to_string());
                }
            }
        }
        // Also add latest mysql (no version suffix)
        if text.lines().any(|l| l.trim() == "mysql") {
            // Get actual version of `mysql` formula
            if let Ok(v) = get_brew_version("mysql") {
                versions.insert(v);
            }
        }

        let mut sorted: Vec<RemoteVersion> = versions.into_iter()
            .map(|v| RemoteVersion { version: v }).collect();
        sorted.sort_by(|a, b| b.version.cmp(&a.version));

        if sorted.is_empty() {
            return Err("No MySQL versions found via Homebrew".into());
        }
        Ok(sorted)
    }

    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        let formula = if version.starts_with("9.") || !version.contains('.') {
            "mysql".to_string()
        } else {
            format!("mysql@{}", version)
        };

        crate::providers::homebrew::brew_ensure(&formula)?;
        let actual = crate::providers::homebrew::brew_version(&formula)?;
        let brew_path = crate::providers::homebrew::brew_prefix(&formula)?;
        let _ = std::fs::create_dir_all(dest);

        for dir in &["bin", "lib", "share"] {
            crate::providers::homebrew::brew_symlink_dir(&brew_path, dest, dir)?;
        }

        eprintln!("MySQL {} linked from {}", actual, brew_path);
        Ok(actual)
    }

    // ── Service Adapter (works with Homebrew-installed mysqld) ────────

    pub fn init_data_dir(install_path: &Path, data_dir: &Path) -> Result<(), String> {
        // Already initialized if mysql system tables exist
        if data_dir.join("mysql").exists() || data_dir.join("ibdata1").exists() {
            return Ok(());
        }
        eprintln!("Initializing MySQL data directory...");
        let mysqld = find_mysqld(install_path)?;
        let status = Command::new(&mysqld)
            .args(["--initialize-insecure", &format!("--datadir={}", data_dir.display())])
            .output()
            .map_err(|e| format!("mysqld init failed: {}", e))?;
        if !status.status.success() {
            return Err(format!("mysqld init: {}", String::from_utf8_lossy(&status.stderr)));
        }
        Ok(())
    }

    pub fn start_service(install_path: &Path, data_dir: &Path, port: u16, socket: &Path) -> Result<RunningService, String> {
        let mysqld = find_mysqld(install_path)?;
        let log_file = data_dir.join("mysql.log");
        let pid_file = data_dir.join("mysql.pid");
        let user = std::env::var("USER").unwrap_or_else(|_| "root".into());

        let child = Command::new(&mysqld)
            .args([
                &format!("--datadir={}", data_dir.display()),
                &format!("--port={}", port),
                &format!("--socket={}", socket.display()),
                &format!("--log-error={}", log_file.display()),
                &format!("--pid-file={}", pid_file.display()),
                &format!("--user={}", user),
                &format!("--basedir={}", install_path.display()),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("mysqld: {}", e))?;

        Ok(RunningService {
            module_name: "mysql".into(), version: String::new(),
            pid: child.id(), port, started_at: Utc::now(),
        })
    }

    pub fn stop_service(_pid: u32) -> Result<(), String> {
        // Try graceful shutdown via mysqladmin first
        eprintln!("Stopping MySQL...");
        let _ = Command::new("mysqladmin")
            .args(["-u", "root", "-h", "127.0.0.1", "shutdown"])
            .output();
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Check if still running
        if let Ok(output) = Command::new("pgrep").args(["-x", "mysqld"]).output() {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.lines() {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    let _ = nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(pid),
                        nix::sys::signal::Signal::SIGTERM,
                    );
                }
            }
        }
        Ok(())
    }

    pub fn read_logs(data_dir: &Path, lines: usize) -> Result<Vec<String>, String> {
        let log_file = data_dir.join("mysql.log");
        if !log_file.exists() { return Ok(vec!["(no log file)".into()]); }
        let content = std::fs::read_to_string(&log_file).map_err(|e| format!("read: {}", e))?;
        let all: Vec<&str> = content.lines().collect();
        let start = all.len().saturating_sub(lines);
        Ok(all[start..].iter().map(|s| s.to_string()).collect())
    }
}

/// Run brew install only if formula is not already installed.
fn brew_ensure(formula: &str) -> Result<(), String> {
    let check = Command::new("brew").args(["--prefix", formula]).output();
    if check.map_or(false, |o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty()) {
        eprintln!("{} already installed, linking...", formula);
        return Ok(());
    }
    eprintln!("Installing {} via Homebrew...", formula);
    let status = Command::new("brew").args(["install", formula])
        .stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit())
        .status().map_err(|e| format!("brew: {}", e))?;
    if !status.success() { eprintln!("brew link had conflicts (ignored)"); }
    Ok(())
}

fn get_brew_prefix(formula: &str) -> Result<String, String> {
    let output = Command::new("brew").args(["--prefix", formula]).output()
        .map_err(|e| format!("brew --prefix: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn brew_symlink(brew_path: &str, dest: &std::path::Path, dir: &str) -> Result<(), String> {
    let src = std::path::PathBuf::from(brew_path).join(dir);
    if src.exists() {
        let dst = dest.join(dir);
        let _ = std::fs::remove_dir_all(&dst);
        let _ = std::fs::remove_file(&dst);
        std::os::unix::fs::symlink(&src, &dst)
            .map_err(|e| format!("symlink {}: {}", dir, e))?;
    }
    Ok(())
}

fn get_brew_version(formula: &str) -> Result<String, String> {
    let output = Command::new("brew")
        .args(["info", "--json=v2", formula])
        .output()
        .map_err(|e| format!("brew info: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|_| "brew info parse error".to_string())?;
    json["formulae"][0]["versions"]["stable"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "version not found".to_string())
}

fn find_mysqld(install_path: &Path) -> Result<std::path::PathBuf, String> {
    for subdir in &["bin", "sbin", "libexec"] {
        let path = install_path.join(subdir).join("mysqld");
        if path.exists() { return Ok(path); }
    }
    Err(format!("mysqld not found in {}", install_path.display()))
}
