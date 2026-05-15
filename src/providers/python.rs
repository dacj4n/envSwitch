//! Python provider — delegates to Homebrew, envswitch manages shim symlinks

use crate::domain::RemoteVersion;
use std::collections::BTreeSet;

pub struct PythonProvider;

impl PythonProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = std::process::Command::new("brew")
            .args(["search", "python"])
            .output()
            .map_err(|_| "Homebrew not found".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions = BTreeSet::new();

        for line in text.lines() {
            let line = line.trim();
            if let Some(ver) = line.strip_prefix("python@3.") {
                let ver = ver.split_whitespace().next().unwrap_or(ver);
                if ver.chars().all(|c| c.is_ascii_digit()) {
                    versions.insert(format!("3.{}", ver));
                }
            }
        }

        // Also check already installed
        if let Ok(out) = std::process::Command::new("brew").args(["list", "--formula"]).output() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(ver) = line.trim().strip_prefix("python@3.") {
                    if ver.chars().all(|c| c.is_ascii_digit()) {
                        versions.insert(format!("3.{}", ver));
                    }
                }
            }
        }

        let mut sorted: Vec<RemoteVersion> = versions.into_iter()
            .map(|v| RemoteVersion { version: v }).collect();
        sorted.sort_by(|a, b| b.version.cmp(&a.version));

        if sorted.is_empty() {
            return Err("No Python versions found via Homebrew".into());
        }
        Ok(sorted)
    }

    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        let formula = format!("python@{}", version);

        eprintln!("Installing {} via Homebrew...", formula);
        let status = std::process::Command::new("brew")
            .args(["install", "--force", &formula])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("brew: {}", e))?;

        if !status.success() {
            eprintln!("brew link had conflicts (ignored)");
        }

        // Get actual version
        let actual = get_brew_version(&formula)?;
        if actual != version {
            eprintln!("Note: installed {} (requested {})", actual, version);
        }

        // Symlink Homebrew's bin into envswitch
        let output = std::process::Command::new("brew")
            .args(["--prefix", &formula])
            .output()
            .map_err(|e| format!("brew --prefix: {}", e))?;

        let brew_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let _ = std::fs::create_dir_all(dest);
        let dest_bin = dest.join("bin");
        let _ = std::fs::remove_dir_all(&dest_bin);
        let _ = std::fs::remove_file(&dest_bin);

        let brew_bin = std::path::PathBuf::from(&brew_path).join("bin");
        std::os::unix::fs::symlink(&brew_bin, &dest_bin)
            .map_err(|e| format!("symlink: {}", e))?;

        // Create `python` -> `python3` symlink (Homebrew only provides python3)
        let python_link = dest_bin.join("python");
        let _ = std::fs::remove_file(&python_link);
        std::os::unix::fs::symlink("python3", &python_link).ok();

        eprintln!("Python {} linked from {}", actual, brew_path);
        Ok(actual)
    }
}

fn get_brew_version(formula: &str) -> Result<String, String> {
    let output = std::process::Command::new("brew")
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
