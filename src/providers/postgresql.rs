//! PostgreSQL provider — Homebrew for installation, envswitch for service management

use crate::domain::{RemoteVersion, RunningService};
use chrono::Utc;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub struct PostgresqlProvider;

impl PostgresqlProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let mut cmd = super::homebrew::brew_cmd();
        let output = cmd
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

        let mut sorted: Vec<RemoteVersion> = versions
            .into_iter()
            .map(|v| RemoteVersion { version: v })
            .collect();
        sorted.sort_by(|a, b| b.version.cmp(&a.version));

        if sorted.is_empty() {
            return Err("No PostgreSQL versions found via Homebrew".into());
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
        let formula = format!("postgresql@{}", version);

        crate::providers::homebrew::brew_ensure_log(&formula, log_tx)?;
        let actual = crate::providers::homebrew::brew_version(&formula)?;
        let brew_path = crate::providers::homebrew::brew_prefix(&formula)?;

        // Single symlink: dest → /opt/homebrew/opt/postgresql@X
        if dest.exists() {
            let _ = std::fs::remove_dir_all(dest);
            let _ = std::fs::remove_file(dest);
        }
        let _ = std::fs::create_dir_all(dest.parent().unwrap());
        std::os::unix::fs::symlink(&brew_path, dest)
            .map_err(|e| format!("symlink {} -> {}: {}", brew_path, dest.display(), e))?;

        eprintln!("PostgreSQL {} linked from {}", actual, brew_path);
        Ok(actual)
    }

    // ── Service Adapter ──────────────────────────────────────────────

    pub fn init_data_dir(install_path: &Path, data_dir: &Path) -> Result<(), String> {
        if data_dir.join("PG_VERSION").exists() {
            return Ok(());
        }
        eprintln!("Initializing PostgreSQL data directory...");
        let initdb = find_binary(install_path, "initdb")?;
        let status = Command::new(&initdb)
            .args(["-D", &data_dir.to_string_lossy()])
            .output()
            .map_err(|e| format!("initdb: {}", e))?;
        if !status.status.success() {
            return Err(format!(
                "initdb failed: {}",
                String::from_utf8_lossy(&status.stderr)
            ));
        }
        Ok(())
    }

    pub fn start_service(
        install_path: &Path,
        data_dir: &Path,
        port: u16,
        _socket: &Path,
    ) -> Result<RunningService, String> {
        let pg_ctl = find_binary(install_path, "pg_ctl")?;
        let log_file = data_dir.join("postgresql.log");
        let pid_file = data_dir.join("postmaster.pid");

        // Clean stale PID file
        if pid_file.exists() {
            if let Ok(old_pid_str) = std::fs::read_to_string(&pid_file) {
                if let Some(old_pid) = old_pid_str.lines().next().and_then(|l| l.trim().parse::<i32>().ok()) {
                    // Check if old process is still running
                    if nix::sys::signal::kill(nix::unistd::Pid::from_raw(old_pid), None).is_ok() {
                        return Err(format!(
                            "PostgreSQL is already running (PID: {}). Stop it first with: envswitch stop pgsql",
                            old_pid
                        ));
                    }
                }
            }
            // Stale — remove it
            let _ = std::fs::remove_file(&pid_file);
        }

        let log_f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| format!("Cannot open log file: {}", e))?;

        let mut child = Command::new(&pg_ctl)
            .args([
                "start",
                "-D",
                &data_dir.to_string_lossy(),
                "-l",
                &log_file.to_string_lossy(),
                "-o",
                &format!("-p {}", port),
            ])
            .stdout(log_f.try_clone().unwrap())
            .stderr(log_f)
            .spawn()
            .map_err(|e| format!("pg_ctl: {}", e))?;

        // pg_ctl start backgrounds postgres, then exits. Wait for both.
        std::thread::sleep(std::time::Duration::from_secs(2));
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => {
                let tail = Self::read_logs(data_dir, 20).unwrap_or_default();
                return Err(format!(
                    "pg_ctl exited with status {}. Last log lines:\n{}",
                    status,
                    tail.join("\n")
                ));
            }
            Ok(None) => {
                // pg_ctl still running (should not happen normally)
            }
            _ => {}
        }

        // Read the real postgres PID from postmaster.pid
        let pid = if pid_file.exists() {
            std::fs::read_to_string(&pid_file)
                .ok()
                .and_then(|s| s.lines().next()?.trim().parse::<u32>().ok())
                .unwrap_or(child.id())
        } else {
            return Err("PostgreSQL did not start — no postmaster.pid found. Check logs.".into());
        };

        // Verify the PID is actually running
        if nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_err() {
            let tail = Self::read_logs(data_dir, 20).unwrap_or_default();
            return Err(format!(
                "PostgreSQL PID {} not alive after start. Log:\n{}",
                pid,
                tail.join("\n")
            ));
        }

        Ok(RunningService {
            module_name: "pgsql".into(),
            version: String::new(),
            pid,
            port,
            started_at: Utc::now(),
        })
    }

    pub fn stop_service(_pid: u32) -> Result<(), String> {
        eprintln!("Stopping PostgreSQL...");
        // Try brew services first
        for f in &[
            "postgresql",
            "postgresql@12",
            "postgresql@13",
            "postgresql@14",
            "postgresql@15",
            "postgresql@16",
            "postgresql@17",
            "postgresql@18",
        ] {
            let mut svc_cmd = super::homebrew::brew_cmd();
            let _ = svc_cmd.args(["services", "stop", f]).output();
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
        if !log_file.exists() {
            return Ok(vec!["(no log file)".into()]);
        }
        let content = std::fs::read_to_string(&log_file).map_err(|e| format!("read: {}", e))?;
        let all: Vec<&str> = content.lines().collect();
        let start = all.len().saturating_sub(lines);
        Ok(all[start..].iter().map(|s| s.to_string()).collect())
    }
}

fn find_binary(install_path: &Path, name: &str) -> Result<std::path::PathBuf, String> {
    for subdir in &["bin", "sbin", "libexec"] {
        let path = install_path.join(subdir).join(name);
        if path.exists() {
            return Ok(path);
        }
    }
    Err(format!("{} not found in {}", name, install_path.display()))
}
