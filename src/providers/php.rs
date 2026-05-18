//! PHP provider — delegates to Homebrew for installation
//! Versions detected via `brew search`, install via `brew install`, symlink for switching.

use crate::domain::RemoteVersion;
use std::collections::BTreeSet;

pub struct PhpProvider;

impl PhpProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let mut cmd = super::homebrew::brew_cmd();
        let output = cmd
            .args(["search", "php"])
            .output()
            .map_err(|_| "Homebrew not found. Install from https://brew.sh".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions = BTreeSet::new();

        for line in text.lines() {
            let line = line.trim();
            if let Some(ver) = line.split("php@").nth(1) {
                let ver = ver.split_whitespace().next().unwrap_or(ver);
                if !ver.is_empty() && ver.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    versions.insert(ver.to_string());
                }
            }
        }

        let mut result: Vec<RemoteVersion> = versions
            .into_iter()
            .map(|v| RemoteVersion { version: v })
            .collect();
        result.sort_by(|a, b| b.version.cmp(&a.version));

        if result.is_empty() {
            return Err(
                "No PHP versions found. Tap shivammathur: brew tap shivammathur/php".into(),
            );
        }
        Ok(result)
    }

    /// Install PHP via Homebrew and symlink into envswitch.
    /// Returns the actual installed version (may differ from requested).
    #[allow(dead_code)]
    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        Self::install_log(version, dest, None)
    }
    pub fn install_log(
        version: &str,
        dest: &std::path::Path,
        log_tx: Option<&std::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        let formula = determine_formula(version)?;

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
            let status = cmd.status().map_err(|e| format!("brew install: {}", e))?;
            if !status.success() {
                eprintln!("brew link had conflicts (ignored)");
            }
        }

        // Get actual installed version from brew
        let actual_version = get_formula_version(&formula)?;
        if actual_version != version {
            eprintln!(
                "Note: installed version is {} (requested {})",
                actual_version, version
            );
        }

        link_brew_to_envswitch(&formula, dest)?;
        Ok(actual_version)
    }
}

fn determine_formula(version: &str) -> Result<String, String> {
    // Extract base major.minor from full version (7.0.33-zts → 7.0)
    let base = version.split('.').take(2).collect::<Vec<_>>().join(".");

    // Check core formula first
    let core = format!("php@{}", base);
    if brew_exists(&core) {
        return Ok(core);
    }

    // Check shivammathur tap
    let tap = format!("shivammathur/php/php@{}", base);
    if brew_exists(&tap) {
        return Ok(tap);
    }

    // Build full version string with variant: 7.0.33-zts
    // Check if the full version (with variant suffix) matches a formula
    if version != base && version.contains('-') {
        let variant = version.split_once('-').map(|x| x.1).unwrap_or("");
        let core_var = format!("php@{}-{}", base, variant);
        if brew_exists(&core_var) {
            return Ok(core_var);
        }
        let tap_var = format!("shivammathur/php/php@{}-{}", base, variant);
        if brew_exists(&tap_var) {
            return Ok(tap_var);
        }
    }

    // Default: use the tap (shivammathur for old versions)
    if brew_exists(&tap) {
        return Ok(tap);
    }
    Ok(core)
}

fn get_formula_version(formula: &str) -> Result<String, String> {
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

fn brew_exists(formula: &str) -> bool {
    super::homebrew::brew_cmd()
        .args(["--prefix", formula])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn link_brew_to_envswitch(formula: &str, dest: &std::path::Path) -> Result<(), String> {
    let mut prefix_cmd = super::homebrew::brew_cmd();
    let output = prefix_cmd.args(["--prefix", formula])
        .output()
        .map_err(|e| format!("brew --prefix: {}", e))?;

    let brew_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if brew_path.is_empty() {
        return Err(format!(
            "Could not find Homebrew install path for {}",
            formula
        ));
    }

    let _ = std::fs::create_dir_all(dest);
    let dest_bin = dest.join("bin");
    let _ = std::fs::remove_dir_all(&dest_bin);
    let _ = std::fs::remove_file(&dest_bin);

    let brew_bin = std::path::PathBuf::from(&brew_path).join("bin");
    if !brew_bin.exists() {
        return Err(format!("{} installed but no bin/ directory found", formula));
    }

    std::os::unix::fs::symlink(&brew_bin, &dest_bin).map_err(|e| {
        format!(
            "symlink {} -> {} failed: {}",
            brew_bin.display(),
            dest_bin.display(),
            e
        )
    })?;

    // Also link sbin if it exists
    let brew_sbin = std::path::PathBuf::from(&brew_path).join("sbin");
    if brew_sbin.exists() {
        let dest_sbin = dest.join("sbin");
        let _ = std::fs::remove_dir_all(&dest_sbin);
        let _ = std::fs::remove_file(&dest_sbin);
        let _ = std::os::unix::fs::symlink(&brew_sbin, &dest_sbin);
    }

    eprintln!("PHP linked: {} -> {}", dest.display(), brew_path);
    Ok(())
}
