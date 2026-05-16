//! Node.js provider — delegates to fnm, no symlinks needed

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
                let l = l.trim();
                if l.starts_with('v') && l.len() > 1 {
                    Some(RemoteVersion { version: l.to_string() })
                } else { None }
            })
            .collect();
        versions.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(versions)
    }

    /// Run fnm install for a version.
    pub fn install(version: &str, _dest: &std::path::Path) -> Result<String, String> {
        let ver = version.trim_start_matches('v');
        eprintln!("Installing Node {} via fnm...", ver);
        let status = Command::new("fnm")
            .args(["install", ver])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("fnm install: {}", e))?;
        if !status.success() {
            return Err(format!("fnm install {} failed", ver));
        }
        Ok(format!("v{}", ver))
    }

    /// Output fnm use command (eval'd by shell function for immediate effect).
    pub fn cover_script(version: &str) -> Result<String, String> {
        let ver = version.trim_start_matches('v');
        // fnm use handles everything — shell function will eval this
        Ok(format!("fnm use {}", ver))
    }
}
