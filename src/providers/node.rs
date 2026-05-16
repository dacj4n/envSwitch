//! Node.js provider — delegates to fnm, version strings without v prefix

use crate::domain::RemoteVersion;
use std::process::Command;

pub struct NodeProvider;

impl NodeProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = Command::new("fnm")
            .args(["list-remote"])
            .output()
            .map_err(|_| "fnm not found. Install: brew install fnm".to_string())?;

        let mut versions: Vec<RemoteVersion> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|l| {
                let l = l.trim().strip_prefix('v')?;
                if !l.is_empty() && l.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                    Some(RemoteVersion { version: l.to_string() })
                } else { None }
            })
            .collect();
        versions.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(versions)
    }

    /// Run fnm install, return version without v prefix.
    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        eprintln!("Installing Node {} via fnm...", version);
        let status = Command::new("fnm")
            .args(["install", version])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("fnm install: {}", e))?;
        if !status.success() {
            return Err(format!("fnm install {} failed", version));
        }
        let _ = std::fs::create_dir_all(dest);
        Ok(version.to_string())
    }

    /// Output fnm use command (eval'd by shell function).
    pub fn cover_script(version: &str) -> Result<String, String> {
        Ok(format!("fnm use {}", version))
    }
}
