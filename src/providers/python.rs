//! Python provider — delegates to Homebrew, envswitch manages shim symlinks

use crate::domain::RemoteVersion;
use std::collections::BTreeSet;

pub struct PythonProvider;

impl PythonProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let brew = if std::path::Path::new("/opt/homebrew/bin/brew").exists() { "/opt/homebrew/bin/brew" } else { "brew" };
        let mut cmd = std::process::Command::new(brew);
        crate::config::apply_proxy(&mut cmd);
        let output = cmd.args(["search", "python"])
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
        Self::install_log(version, dest, None)
    }
    pub fn install_log(version: &str, dest: &std::path::Path, log_tx: Option<&std::sync::mpsc::Sender<String>>) -> Result<String, String> {
        let formula = format!("python@{}", version);

        let msg = format!("brew install --force {}", formula);
        if let Some(tx) = log_tx { let _ = tx.send(msg.clone()); }
        eprintln!("{}", msg);
        let mut cmd = std::process::Command::new("brew");
        crate::config::apply_proxy(&mut cmd);
        cmd.args(["install", "--force", &formula]);
        if let Some(tx) = log_tx {
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let mut child = cmd.spawn().map_err(|e| format!("brew: {}", e))?;
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let tx1 = tx.clone(); let tx2 = tx.clone();
            std::thread::spawn(move || { use std::io::BufRead; for l in std::io::BufReader::new(stdout).lines().flatten() { let _ = tx1.send(l); } });
            std::thread::spawn(move || { use std::io::BufRead; for l in std::io::BufReader::new(stderr).lines().flatten() { let _ = tx2.send(l); } });
            let status = child.wait().map_err(|e| format!("brew wait: {}", e))?;
            if !status.success() { eprintln!("brew link had conflicts (ignored)"); }
        } else {
            cmd.stdout(std::process::Stdio::inherit());
            cmd.stderr(std::process::Stdio::inherit());
            let status = cmd.status().map_err(|e| format!("brew: {}", e))?;
            if !status.success() { eprintln!("brew link had conflicts (ignored)"); }
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

        // Some Homebrew Python versions only provide versioned binaries (python3.11).
        // Create unversioned symlinks so `python` and `python3` work.
        let ver_bin = format!("python3.{}", version.split('.').nth(1).unwrap_or(""));
        let ver_bin_path = dest_bin.join(&ver_bin);
        if ver_bin_path.exists() && !dest_bin.join("python3").exists() {
            std::os::unix::fs::symlink(&ver_bin, dest_bin.join("python3")).ok();
        }
        if dest_bin.join("python3").exists() && !dest_bin.join("python").exists() {
            std::os::unix::fs::symlink("python3", dest_bin.join("python")).ok();
        }

        eprintln!("Python {} linked from {}", actual, brew_path);
        Ok(actual)
    }
}

fn get_brew_version(formula: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new("brew");
    crate::config::apply_proxy(&mut cmd);
    let output = cmd.args(["info", "--json=v2", formula])
        .output()
        .map_err(|e| format!("brew info: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|_| "brew info parse error".to_string())?;
    json["formulae"][0]["versions"]["stable"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "version not found".to_string())
}
