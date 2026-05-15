//! PostgreSQL provider — Homebrew for installation, envswitch for service management

use crate::domain::{RemoteVersion, RunningService};
use chrono::Utc;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub struct PostgresqlProvider;

impl PostgresqlProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = Command::new("brew")
            .args(["search", "postgresql"])
            .output()
            .map_err(|_| "Homebrew not found".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions = BTreeSet::new();

        for line in text.lines() {
            if let Some(ver) = line.trim().strip_prefix("postgresql@") {
                let ver = ver.split_whitespace().next().unwrap_or(ver);
                if ver.chars().all(|c| c.is_ascii_digit()) {
                    versions.insert(ver.to_string());
                }
            }
        }

        // Also add latest (no version suffix)
        if text.lines().any(|l| l.trim() == "postgresql") {
            if let Ok(v) = crate::providers::homebrew::brew_version("postgresql") {
                versions.insert(v);
            }
        }

        // Batch query brew info for full version numbers
        let full_versions = batch_brew_versions(&versions);

        let mut sorted: Vec<RemoteVersion> = versions.into_iter()
            .map(|v| {
                let full = full_versions.get(&v).cloned().unwrap_or(v);
                RemoteVersion { version: full }
            })
            .collect();
        sorted.sort_by(|a, b| b.version.cmp(&a.version));

        if sorted.is_empty() {
            return Err("No PostgreSQL versions found via Homebrew".into());
        }
        Ok(sorted)
    }

    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        let formula = if version.contains('.') {
            format!("postgresql@{}", version)
        } else {
            format!("postgresql@{}", version)
        };

        crate::providers::homebrew::brew_ensure(&formula)?;
        let actual = crate::providers::homebrew::brew_version(&formula)?;
        let brew_path = crate::providers::homebrew::brew_prefix(&formula)?;
        let _ = std::fs::create_dir_all(dest);

        for dir in &["bin", "lib", "share"] {
            crate::providers::homebrew::brew_symlink_dir(&brew_path, dest, dir)?;
        }

        eprintln!("PostgreSQL {} linked from {}", actual, brew_path);
        Ok(actual)
    }

    // ── Service Adapter ──────────────────────────────────────────────

    pub fn init_data_dir(_install_path: &Path, data_dir: &Path) -> Result<(), String> {
        if data_dir.join("PG_VERSION").exists() { return Ok(()); }
        eprintln!("Initializing PostgreSQL data directory...");
        let status = Command::new("initdb")
            .args(["-D", &data_dir.to_string_lossy()])
            .output()
            .map_err(|e| format!("initdb: {}", e))?;
        if !status.status.success() {
            return Err(format!("initdb failed: {}", String::from_utf8_lossy(&status.stderr)));
        }
        Ok(())
    }

    pub fn start_service(install_path: &Path, data_dir: &Path, port: u16, _socket: &Path) -> Result<RunningService, String> {
        let pg_ctl = find_binary(install_path, "pg_ctl")?;
        let log_file = data_dir.join("postgresql.log");

        let child = Command::new(&pg_ctl)
            .args([
                "start", "-D", &data_dir.to_string_lossy(),
                "-l", &log_file.to_string_lossy(),
                "-o", &format!("-p {}", port),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("pg_ctl: {}", e))?;

        // pg_ctl start returns immediately, get the actual postgres PID
        std::thread::sleep(std::time::Duration::from_secs(1));
        let pid_file = data_dir.join("postmaster.pid");
        let pid = if pid_file.exists() {
            std::fs::read_to_string(&pid_file)
                .ok()
                .and_then(|s| s.lines().next()?.trim().parse().ok())
                .unwrap_or(child.id())
        } else {
            child.id()
        };

        Ok(RunningService {
            module_name: "pgsql".into(), version: String::new(),
            pid, port, started_at: Utc::now(),
        })
    }

    pub fn stop_service(_pid: u32) -> Result<(), String> {
        eprintln!("Stopping PostgreSQL...");
        // Try brew services first
        for f in &["postgresql", "postgresql@12", "postgresql@13", "postgresql@14",
                    "postgresql@15", "postgresql@16", "postgresql@17", "postgresql@18"] {
            let _ = Command::new("brew").args(["services", "stop", f]).output();
        }
        // Force kill remaining
        if let Ok(output) = Command::new("pgrep").args(["-x", "postgres"]).output() {
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
        let log_file = data_dir.join("postgresql.log");
        if !log_file.exists() { return Ok(vec!["(no log file)".into()]); }
        let content = std::fs::read_to_string(&log_file).map_err(|e| format!("read: {}", e))?;
        let all: Vec<&str> = content.lines().collect();
        let start = all.len().saturating_sub(lines);
        Ok(all[start..].iter().map(|s| s.to_string()).collect())
    }
}

fn batch_brew_versions(versions: &BTreeSet<String>) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let mut args: Vec<String> = vec!["info".into(), "--json=v2".into()];
    for v in versions {
        args.push(format!("postgresql@{}", v));
    }

    if let Ok(output) = Command::new("brew").args(&args).output() {
        if output.status.success() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout)) {
                if let Some(formulae) = json["formulae"].as_array() {
                    for f in formulae {
                        if let (Some(name), Some(ver)) = (
                            f["name"].as_str(),
                            f["versions"]["stable"].as_str(),
                        ) {
                            if let Some(short) = name.strip_prefix("postgresql@") {
                                map.insert(short.to_string(), ver.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

fn find_binary(install_path: &Path, name: &str) -> Result<std::path::PathBuf, String> {
    for subdir in &["bin", "sbin", "libexec"] {
        let path = install_path.join(subdir).join(name);
        if path.exists() { return Ok(path); }
    }
    Err(format!("{} not found in {}", name, install_path.display()))
}
