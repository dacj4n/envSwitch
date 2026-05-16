//! Node.js provider — delegates to fnm (Fast Node Manager) for version management

use crate::domain::RemoteVersion;

pub struct NodeProvider;

impl NodeProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = std::process::Command::new("fnm")
            .args(["list-remote"])
            .output()
            .map_err(|_| "fnm not found. Install: brew install fnm".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions: Vec<RemoteVersion> = text
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with('v') && line.len() > 1 {
                    Some(RemoteVersion { version: line.to_string() })
                } else {
                    None
                }
            })
            .collect();

        versions.sort_by(|a, b| b.version.cmp(&a.version));
        if versions.is_empty() {
            return Err("No Node.js versions found. Check fnm: fnm list-remote".into());
        }
        Ok(versions)
    }

    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        let ver = if version.starts_with('v') { version.to_string() } else { format!("v{}", version) };

        // Check if fnm already has this version
        let fnm_versions_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".local/share/fnm/node-versions");
        let fnm_dir = fnm_versions_dir.join(&ver).join("installation");

        if !fnm_dir.exists() {
            eprintln!("Installing Node {} via fnm...", ver);
            let status = std::process::Command::new("fnm")
                .args(["install", &ver.trim_start_matches('v')])
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .map_err(|e| format!("fnm install: {}", e))?;
            if !status.success() {
                return Err(format!("fnm install {} failed", ver));
            }
        } else {
            eprintln!("Node {} already installed via fnm, linking...", ver);
        }

        // Symlink fnm's installation into envswitch
        let _ = std::fs::create_dir_all(dest);
        for dir in &["bin"] {
            let src = fnm_dir.join(dir);
            if src.exists() {
                let dst = dest.join(dir);
                let _ = std::fs::remove_dir_all(&dst);
                let _ = std::fs::remove_file(&dst);
                std::os::unix::fs::symlink(&src, &dst)
                    .map_err(|e| format!("symlink {}: {}", dir, e))?;
            }
        }

        eprintln!("Node {} linked", ver);
        Ok(ver)
    }
}
