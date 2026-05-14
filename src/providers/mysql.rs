use crate::domain::{ArchiveFormat, ChecksumType, RunningService};
use crate::infra::download;
use crate::platform::Platform;
use chrono::Utc;
use std::path::Path;
use std::process::Command;

pub struct MySqlProvider;

impl MySqlProvider {
    pub fn fetch_remote_versions() -> Result<Vec<String>, String> {
        // MySQL doesn't have a simple public JSON API for versions.
        Ok(vec![
            "8.0.37".into(), "8.0.36".into(), "8.0.35".into(),
            "8.4.1".into(), "8.4.0".into(),
            "5.7.44".into(),
        ])
    }

    pub fn download_url(version: &str) -> Result<String, String> {
        let platform = Platform::current();
        let tag = platform.mysql_os_tag();
        let ver_dir = Platform::mysql_version_dir(version);

        // MySQL 5.7 doesn't support ARM64
        if version.starts_with("5.") && tag.contains("arm64") {
            return Err(format!(
                "MySQL {} does not support ARM64 (Apple Silicon).\n\
                 Use MySQL 8.0+ instead: envswitch install mysql 8.0.37",
                version
            ));
        }

        // Use CDN URL (dev.mysql.com/get redirects to HTML page)
        Ok(format!(
            "https://cdn.mysql.com/Downloads/{}/mysql-{}-{}.tar.gz",
            ver_dir, version, tag
        ))
    }

    pub fn checksum_url(_version: &str) -> Option<String> { None }

    pub fn archive_format() -> ArchiveFormat { ArchiveFormat::TarGz }

    pub fn checksum_type() -> ChecksumType { ChecksumType::None }

    pub fn install(archive: &Path, dest: &Path) -> Result<(), String> {
        download::extract_archive(archive, dest, &ArchiveFormat::TarGz)
    }

    // ── Service Adapter ──────────────────────────────────────────────

    pub fn init_data_dir(install_path: &Path, data_dir: &Path) -> Result<(), String> {
        if data_dir.join("mysql").exists() {
            return Ok(());
        }
        let mysqld = find_mysqld(install_path)?;
        let status = Command::new(&mysqld)
            .args([
                "--initialize-insecure",
                &format!("--datadir={}", data_dir.display()),
            ])
            .output()
            .map_err(|e| format!("mysqld init failed: {}", e))?;
        if !status.status.success() {
            return Err(format!("mysqld init failed: {}", String::from_utf8_lossy(&status.stderr)));
        }
        Ok(())
    }

    pub fn start_service(install_path: &Path, data_dir: &Path, port: u16) -> Result<RunningService, String> {
        let mysqld = find_mysqld(install_path)?;
        let log_file = data_dir.join("mysql.log");
        let pid_file = data_dir.join("mysql.pid");

        let child = Command::new(&mysqld)
            .args([
                &format!("--datadir={}", data_dir.display()),
                &format!("--port={}", port),
                &format!("--socket={}", data_dir.join("mysql.sock").display()),
                &format!("--log-error={}", log_file.display()),
                &format!("--pid-file={}", pid_file.display()),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start mysqld: {}", e))?;

        Ok(RunningService {
            module_name: "mysql".into(),
            version: String::new(),
            pid: child.id(),
            port,
            started_at: Utc::now(),
        })
    }

    pub fn stop_service(pid: u32) -> Result<(), String> {
        // Try graceful shutdown first
        let _ = Command::new("mysqladmin")
            .args(["-u", "root", "-h", "127.0.0.1", "shutdown"])
            .output();
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Fallback: SIGTERM
        if crate::infra::fs::read_pid_file("mysql").is_some() {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGTERM,
            ).map_err(|e| format!("SIGTERM failed: {}", e))?;
        }
        Ok(())
    }

    pub fn read_logs(data_dir: &Path, lines: usize) -> Result<Vec<String>, String> {
        let log_file = data_dir.join("mysql.log");
        if !log_file.exists() { return Ok(vec!["(no log file)".into()]); }
        let content = std::fs::read_to_string(&log_file)
            .map_err(|e| format!("Cannot read log: {}", e))?;
        let all_lines: Vec<&str> = content.lines().collect();
        let start = if all_lines.len() > lines { all_lines.len() - lines } else { 0 };
        Ok(all_lines[start..].iter().map(|s| s.to_string()).collect())
    }
}

fn find_mysqld(install_path: &Path) -> Result<std::path::PathBuf, String> {
    for subdir in &["bin", "sbin", "libexec"] {
        let path = install_path.join(subdir).join("mysqld");
        if path.exists() { return Ok(path); }
    }
    Err(format!("mysqld not found in {}", install_path.display()))
}
