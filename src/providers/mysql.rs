//! MySQL provider — Homebrew for installation, envswitch for service management

use crate::domain::{RemoteVersion, RunningService};
use chrono::Utc;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub struct MySqlProvider;

impl MySqlProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let mut cmd = super::homebrew::brew_cmd();
        let output = cmd
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

        let mut sorted: Vec<RemoteVersion> = versions
            .into_iter()
            .map(|v| RemoteVersion { version: v })
            .collect();
        sorted.sort_by(|a, b| b.version.cmp(&a.version));

        if sorted.is_empty() {
            return Err("No MySQL versions found via Homebrew".into());
        }
        Ok(sorted)
    }

    #[allow(dead_code)]
    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        Self::install_log(version, dest, None)
    }
    pub fn install_log(
        version: &str,
        dest: &std::path::Path,
        log_tx: Option<&std::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        let formula = if version.starts_with("9.") || !version.contains('.') {
            "mysql".to_string()
        } else {
            format!("mysql@{}", version)
        };

        crate::providers::homebrew::brew_ensure_log(&formula, log_tx)?;
        let actual = crate::providers::homebrew::brew_version(&formula)?;
        let brew_path = crate::providers::homebrew::brew_prefix(&formula)?;

        // Single symlink: dest → /opt/homebrew/opt/mysql@X.Y
        if dest.exists() {
            let _ = std::fs::remove_dir_all(dest);
            let _ = std::fs::remove_file(dest);
        }
        let _ = std::fs::create_dir_all(dest.parent().unwrap());
        std::os::unix::fs::symlink(&brew_path, dest)
            .map_err(|e| format!("symlink {} -> {}: {}", brew_path, dest.display(), e))?;

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
            .args([
                "--initialize-insecure",
                &format!("--datadir={}", data_dir.display()),
            ])
            .output()
            .map_err(|e| format!("mysqld init failed: {}", e))?;
        if !status.status.success() {
            return Err(format!(
                "mysqld init: {}",
                String::from_utf8_lossy(&status.stderr)
            ));
        }
        Ok(())
    }

    pub fn start_service(
        install_path: &Path,
        data_dir: &Path,
        port: u16,
        socket: &Path,
    ) -> Result<RunningService, String> {
        // Clean up stale socket files from previous runs
        for sock in &["/tmp/mysql.sock", "/tmp/mysqlx.sock"] {
            let _ = std::fs::remove_file(sock);
        }
        // Clean stale socket lock file
        let lock_file = format!("{}.lock", socket.display());
        let _ = std::fs::remove_file(&lock_file);
        // Kill any leftover mysqld processes
        if let Ok(output) = Command::new("pgrep").args(["-x", "mysqld"]).output() {
            for pid_str in String::from_utf8_lossy(&output.stdout).lines() {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    eprintln!("Killing stale mysqld (PID: {})...", pid);
                    let _ = nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(pid),
                        nix::sys::signal::Signal::SIGKILL,
                    );
                }
            }
            // Wait for processes to die
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        let mysqld = find_mysqld(install_path)?;
        let log_file = data_dir.join("mysql.log");
        let pid_file = data_dir.join("mysql.pid");
        let user = std::env::var("USER").unwrap_or_else(|_| "root".into());

        let log_f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| format!("Cannot open log file: {}", e))?;

        let mut child = Command::new(&mysqld)
            .args([
                &format!("--datadir={}", data_dir.display()),
                &format!("--port={}", port),
                &format!("--socket={}", socket.display()),
                "--mysqlx=0", // Disable X Plugin to avoid /tmp/mysqlx.sock
                &format!("--log-error={}", log_file.display()),
                &format!("--pid-file={}", pid_file.display()),
                &format!("--user={}", user),
                &format!("--basedir={}", install_path.display()),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(log_f)
            .spawn()
            .map_err(|e| format!("mysqld: {}", e))?;

        // Wait and verify the process is still alive
        std::thread::sleep(std::time::Duration::from_secs(2));
        match child.try_wait() {
            Ok(Some(status)) => {
                let tail = Self::read_logs(data_dir, 20).unwrap_or_default();
                return Err(format!(
                    "MySQL exited immediately (status: {}).\nLast log lines:\n{}",
                    status,
                    tail.join("\n")
                ));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(format!("Cannot check mysqld status: {}", e));
            }
        }

        // Symlink /tmp/mysql.sock → version-specific socket (for client compatibility)
        let tmp_sock = std::path::Path::new("/tmp/mysql.sock");
        let _ = std::fs::remove_file(tmp_sock);
        std::os::unix::fs::symlink(socket, tmp_sock).ok();

        Ok(RunningService {
            module_name: "mysql".into(),
            version: String::new(),
            pid: child.id(),
            port,
            started_at: Utc::now(),
        })
    }

    pub fn stop_service(_pid: u32) -> Result<(), String> {
        eprintln!("Stopping MySQL...");
        // Try mysqladmin via TCP
        let _ = Command::new("mysqladmin")
            .args(["-u", "root", "-h", "127.0.0.1", "shutdown"])
            .output();
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Try brew services stop for any running mysql formula
        for f in &["mysql", "mysql@8.0", "mysql@8.4"] {
            let mut svc_cmd = super::homebrew::brew_cmd();
            let _ = svc_cmd.args(["services", "stop", f]).output();
        }

        // Force kill any remaining mysqld
        if let Ok(output) = Command::new("pgrep").args(["-x", "mysqld"]).output() {
            for pid_str in String::from_utf8_lossy(&output.stdout).lines() {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    let _ = nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(pid),
                        nix::sys::signal::Signal::SIGKILL,
                    );
                }
            }
        }
        Ok(())
    }

    pub fn read_logs(data_dir: &Path, lines: usize) -> Result<Vec<String>, String> {
        let log_file = data_dir.join("mysql.log");
        if !log_file.exists() {
            return Ok(vec!["(no log file)".into()]);
        }
        let content = std::fs::read_to_string(&log_file).map_err(|e| format!("read: {}", e))?;
        let all: Vec<&str> = content.lines().collect();
        let start = all.len().saturating_sub(lines);
        Ok(all[start..].iter().map(|s| s.to_string()).collect())
    }
}

fn get_brew_version(formula: &str) -> Result<String, String> {
    let mut cmd = super::homebrew::brew_cmd();
    let output = cmd
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
        if path.exists() {
            return Ok(path);
        }
    }
    Err(format!("mysqld not found in {}", install_path.display()))
}
