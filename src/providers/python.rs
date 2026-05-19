//! Python provider — delegates to Homebrew, envswitch manages shim symlinks

use crate::domain::RemoteVersion;
use std::collections::BTreeSet;

pub struct PythonProvider;

impl PythonProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let mut cmd = super::homebrew::brew_cmd();
        let output = cmd
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
        let mut list_cmd = super::homebrew::brew_cmd();
        if let Ok(out) = list_cmd.args(["list", "--formula"]).output() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(ver) = line.trim().strip_prefix("python@3.") {
                    if ver.chars().all(|c| c.is_ascii_digit()) {
                        versions.insert(format!("3.{}", ver));
                    }
                }
            }
        }

        let mut sorted: Vec<RemoteVersion> = versions
            .into_iter()
            .map(|v| RemoteVersion { version: v })
            .collect();
        sorted.sort_by(|a, b| b.version.cmp(&a.version));

        if sorted.is_empty() {
            return Err("No Python versions found via Homebrew".into());
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
        let formula = format!("python@{}", version);

        let msg = format!("brew install --force {}", formula);
        if let Some(tx) = log_tx {
            let _ = tx.send(msg.clone());
        }
        eprintln!("{}", msg);
        let mut cmd = super::homebrew::brew_cmd();
        cmd.args(["install", "--force", &formula]);
        if let Some(tx) = log_tx {
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let mut child = cmd.spawn().map_err(|e| format!("brew: {}", e))?;
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let tx1 = tx.clone();
            let tx2 = tx.clone();
            std::thread::spawn(move || {
                use std::io::BufRead;
                for l in std::io::BufReader::new(stdout)
                    .lines()
                    .map_while(Result::ok)
                {
                    let _ = tx1.send(l);
                }
            });
            std::thread::spawn(move || {
                use std::io::BufRead;
                for l in std::io::BufReader::new(stderr)
                    .lines()
                    .map_while(Result::ok)
                {
                    let _ = tx2.send(l);
                }
            });
            let status = child.wait().map_err(|e| format!("brew wait: {}", e))?;
            if !status.success() {
                eprintln!("brew link had conflicts (ignored)");
            }
        } else {
            cmd.stdout(std::process::Stdio::inherit());
            cmd.stderr(std::process::Stdio::inherit());
            let status = cmd.status().map_err(|e| format!("brew: {}", e))?;
            if !status.success() {
                eprintln!("brew link had conflicts (ignored)");
            }
        }

        // Get actual version
        let actual = get_brew_version(&formula)?;
        if actual != version {
            eprintln!("Note: installed {} (requested {})", actual, version);
        }

        // Single symlink: dest → /opt/homebrew/opt/python@X.Y
        let brew_path = crate::providers::homebrew::brew_prefix(&formula)?;
        if dest.exists() {
            let _ = std::fs::remove_dir_all(dest);
            let _ = std::fs::remove_file(dest);
        }
        let _ = std::fs::create_dir_all(dest.parent().unwrap());
        std::os::unix::fs::symlink(&brew_path, dest)
            .map_err(|e| format!("symlink {} -> {}: {}", brew_path, dest.display(), e))?;

        // Homebrew python@X.Y is keg-only — python3 / python symlinks are
        // not created by brew link. Add them so shims resolve correctly.
        let brew_bin = std::path::PathBuf::from(&brew_path).join("bin");
        let ver_bin = format!(
            "python3.{}",
            version.split('.').nth(1).unwrap_or("")
        );
        if brew_bin.join(&ver_bin).exists() {
            let py3 = brew_bin.join("python3");
            if !py3.exists() {
                std::os::unix::fs::symlink(&ver_bin, &py3).ok();
            }
            let py = brew_bin.join("python");
            if !py.exists() {
                std::os::unix::fs::symlink("python3", &py).ok();
            }
        }

        eprintln!("Python {} linked from {}", actual, brew_path);
        Ok(actual)
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
