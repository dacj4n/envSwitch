use crate::domain::{ArchiveFormat, ChecksumType, RunningService};
use crate::infra::download;
use chrono::Utc;
use std::path::Path;
use std::process::Command;

/// MySQL provider and service adapter.
pub struct MySqlProvider;

impl MySqlProvider {
    pub fn fetch_remote_versions() -> Result<Vec<String>, String> {
        // MySQL doesn't have a simple JSON API; return known recent versions
        Ok(vec![
            "8.0.35".into(),
            "8.0.34".into(),
            "8.0.33".into(),
            "8.4.0".into(),
            "5.7.44".into(),
        ])
    }

    pub fn download_url(version: &str) -> String {
        let os = if cfg!(target_os = "macos") { "macos14" } else { "linux-glibc2.28" };
        let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };
        // MySQL community server download
        format!(
            "https://dev.mysql.com/get/Downloads/MySQL-8.0/mysql-{}-{}-{}.tar.gz",
            version, os, arch
        )
    }

    pub fn checksum_url(_version: &str) -> Option<String> {
        None // MySQL downloads don't have separate checksum files
    }

    pub fn archive_format() -> ArchiveFormat {
        ArchiveFormat::TarGz
    }

    pub fn checksum_type() -> ChecksumType {
        ChecksumType::None
    }

    pub fn install(archive: &Path, dest: &Path) -> Result<(), String> {
        download::extract_archive(archive, dest, &ArchiveFormat::TarGz)
    }

    // ── Service Adapter methods ─────────────────────────────────────

    /// Initialize MySQL data directory: mysqld --initialize-insecure
    pub fn init_data_dir(install_path: &Path, data_dir: &Path) -> Result<(), String> {
        if data_dir.join("mysql").exists() {
            return Ok(()); // Already initialized
        }

        let mysqld = find_mysqld(install_path)?;
        let status = Command::new(&mysqld)
            .args([
                "--initialize-insecure",
                &format!("--datadir={}", data_dir.display()),
                "--user=root",
            ])
            .output()
            .map_err(|e| format!("Failed to run mysqld --initialize-insecure: {}", e))?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(format!("mysqld init failed: {}", stderr));
        }
        Ok(())
    }

    /// Start MySQL server.
    pub fn start_service(
        install_path: &Path,
        data_dir: &Path,
        port: u16,
    ) -> Result<RunningService, String> {
        let mysqld = find_mysqld(install_path)?;
        let log_file = data_dir.join("mysql.log");

        let child = Command::new(&mysqld)
            .args([
                &format!("--datadir={}", data_dir.display()),
                &format!("--port={}", port),
                &format!("--socket={}", data_dir.join("mysql.sock").display()),
                &format!("--log-error={}", log_file.display()),
                &("--pid-file=".to_string() + &data_dir.join("mysql.pid").display().to_string()),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start mysqld: {}", e))?;

        let pid = child.id();
        Ok(RunningService {
            module_name: "mysql".into(),
            version: "unknown".into(), // filled by caller
            pid,
            port,
            started_at: Utc::now(),
        })
    }

    /// Stop MySQL server.
    pub fn stop_service(pid: u32) -> Result<(), String> {
        // Try graceful shutdown via mysqladmin, then fall back to SIGTERM
        let status = Command::new("mysqladmin")
            .args(["-u", "root", "-h", "127.0.0.1", "shutdown"])
            .output();

        if status.is_ok() {
            // Wait briefly
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        // Fallback: send SIGTERM
        if crate::infra::fs::read_pid_file("mysql").is_some() {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGTERM,
            )
            .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;
        }

        Ok(())
    }

    pub fn read_logs(data_dir: &Path, lines: usize) -> Result<Vec<String>, String> {
        let log_file = data_dir.join("mysql.log");
        if !log_file.exists() {
            return Ok(vec!["(no log file)".into()]);
        }
        let content =
            std::fs::read_to_string(&log_file).map_err(|e| format!("Cannot read log: {}", e))?;
        let all_lines: Vec<&str> = content.lines().collect();
        let start = if all_lines.len() > lines {
            all_lines.len() - lines
        } else {
            0
        };
        Ok(all_lines[start..].iter().map(|s| s.to_string()).collect())
    }
}

fn find_mysqld(install_path: &Path) -> Result<std::path::PathBuf, String> {
    // Common locations within MySQL installation
    for subdir in &["bin", "sbin", "libexec"] {
        let path = install_path.join(subdir).join("mysqld");
        if path.exists() {
            return Ok(path);
        }
    }
    Err(format!(
        "mysqld not found in {}. Please ensure MySQL is installed correctly.",
        install_path.display()
    ))
}
