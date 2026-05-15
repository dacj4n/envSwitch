//! PHP provider — delegates to Homebrew for installation
//! envswitch manages version switching via shim symlinks, not compilation.

use crate::domain::RemoteVersion;

pub struct PhpProvider;

impl PhpProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        // Homebrew PHP versions (major only, brew handles patches)
        Ok(vec![
            "8.4".into(), "8.3".into(), "8.2".into(), "8.1".into(),
        ]
        .into_iter()
        .map(|v| RemoteVersion { version: v })
        .collect())
    }

    /// Install PHP via Homebrew and symlink into envswitch.
    pub fn install(version: &str, dest: &std::path::Path) -> Result<(), String> {
        let formula = format!("php@{}", version);

        // Check if brew is available
        let brew_check = std::process::Command::new("brew")
            .arg("--version")
            .output()
            .map_err(|_| "Homebrew is required for PHP. Install from https://brew.sh".to_string())?;

        if !brew_check.status.success() {
            return Err("Homebrew not found. Install from https://brew.sh".into());
        }

        // Install PHP via Homebrew
        eprintln!("Installing {} via Homebrew...", formula);
        let status = std::process::Command::new("brew")
            .args(["install", &formula])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("brew install failed: {}", e))?;

        if !status.success() {
            return Err(format!("brew install {} failed", formula));
        }

        // Find Homebrew's install path
        let output = std::process::Command::new("brew")
            .args(["--prefix", &formula])
            .output()
            .map_err(|e| format!("brew --prefix: {}", e))?;

        let brew_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if brew_path.is_empty() {
            return Err(format!("Could not find Homebrew install path for {}", formula));
        }

        // Symlink Homebrew's bin/ into envswitch's directory
        let _ = std::fs::create_dir_all(dest);
        let dest_bin = dest.join("bin");
        let _ = std::fs::remove_dir_all(&dest_bin);
        let _ = std::fs::remove_file(&dest_bin);
        let brew_bin = std::path::PathBuf::from(&brew_path).join("bin");
        std::os::unix::fs::symlink(&brew_bin, &dest_bin)
            .map_err(|e| format!("symlink: {}", e))?;

        eprintln!("PHP {} linked from {}", version, brew_path);
        Ok(())
    }
}
