//! PHP provider — delegates to Homebrew for installation
//! Versions detected via `brew search`, install via `brew install`, symlink for switching.

use crate::domain::RemoteVersion;
use std::collections::BTreeSet;

pub struct PhpProvider;

impl PhpProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = std::process::Command::new("brew")
            .args(["search", "php"])
            .output()
            .map_err(|_| "Homebrew not found. Install from https://brew.sh".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions = BTreeSet::new();

        for line in text.lines() {
            let line = line.trim();
            if let Some(ver) = line.split("php@").nth(1) {
                let ver = ver.split_whitespace().next().unwrap_or(ver);
                if !ver.is_empty() && ver.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                    versions.insert(ver.to_string());
                }
            }
        }

        // Batch-query actual full versions via single brew info call
        let full_versions = batch_get_versions(&versions);

        let mut result: Vec<RemoteVersion> = versions
            .into_iter()
            .map(|v| {
                // Combine base version + variant suffix: "8.6-zts" → "8.6.0-zts"
                let base = v.split('-').next().unwrap_or(&v);
                let base_full = full_versions.get(base).cloned().unwrap_or_else(|| base.to_string());
                let suffix = if v.contains('-') {
                    v.splitn(2, '-').nth(1).map(|s| format!("-{}", s)).unwrap_or_default()
                } else {
                    String::new()
                };
                RemoteVersion { version: format!("{}{}", base_full, suffix) }
            })
            .collect();
        result.sort_by(|a, b| b.version.cmp(&a.version));

        if result.is_empty() {
            return Err("No PHP versions found. Tap shivammathur: brew tap shivammathur/php".into());
        }
        Ok(result)
    }

    /// Install PHP via Homebrew and symlink into envswitch.
    /// Returns the actual installed version (may differ from requested).
    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        let formula = determine_formula(version)?;

        eprintln!("Installing {} via Homebrew...", formula);
        let status = std::process::Command::new("brew")
            .args(["install", "--force", &formula])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("brew install: {}", e))?;

        if !status.success() {
            eprintln!("brew link had conflicts (ignored) — envswitch uses own symlinks");
        }

        // Get actual installed version from brew
        let actual_version = get_formula_version(&formula)?;
        if actual_version != version {
            eprintln!("Note: installed version is {} (requested {})", actual_version, version);
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
    if brew_exists(&core) { return Ok(core); }

    // Check shivammathur tap
    let tap = format!("shivammathur/php/php@{}", base);
    if brew_exists(&tap) { return Ok(tap); }

    // Build full version string with variant: 7.0.33-zts
    // Check if the full version (with variant suffix) matches a formula
    if version != base && version.contains('-') {
        let variant = version.splitn(2, '-').nth(1).unwrap_or("");
        let core_var = format!("php@{}-{}", base, variant);
        if brew_exists(&core_var) { return Ok(core_var); }
        let tap_var = format!("shivammathur/php/php@{}-{}", base, variant);
        if brew_exists(&tap_var) { return Ok(tap_var); }
    }

    // Default: use the tap (shivammathur for old versions)
    if brew_exists(&tap) { return Ok(tap); }
    Ok(core)
}

/// Query brew info for all versions in one batch call.
fn batch_get_versions(versions: &BTreeSet<String>) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    // Build formula list: try both core and shivammathur for each version
    let mut args: Vec<String> = vec!["info".into(), "--json=v2".into()];
    for v in versions {
        args.push(format!("php@{}", v));
        args.push(format!("shivammathur/php/php@{}", v));
    }

    if let Ok(output) = std::process::Command::new("brew").args(&args).output() {
        if output.status.success() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout)) {
                if let Some(formulae) = json["formulae"].as_array() {
                    for f in formulae {
                        let name = f["name"].as_str().unwrap_or("");
                        let ver = f["versions"]["stable"].as_str().unwrap_or("");
                        if let Some(short) = name.split("php@").nth(1) {
                            let short = short.split('/').last().unwrap_or(short);
                            if !ver.is_empty() {
                                map.entry(short.to_string()).or_insert_with(|| ver.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

fn get_formula_version(formula: &str) -> Result<String, String> {
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

fn brew_exists(formula: &str) -> bool {
    std::process::Command::new("brew")
        .args(["--prefix", formula])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn link_brew_to_envswitch(formula: &str, dest: &std::path::Path) -> Result<(), String> {
    let output = std::process::Command::new("brew")
        .args(["--prefix", formula])
        .output()
        .map_err(|e| format!("brew --prefix: {}", e))?;

    let brew_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if brew_path.is_empty() {
        return Err(format!("Could not find Homebrew install path for {}", formula));
    }

    let _ = std::fs::create_dir_all(dest);
    let dest_bin = dest.join("bin");
    let _ = std::fs::remove_dir_all(&dest_bin);
    let _ = std::fs::remove_file(&dest_bin);

    let brew_bin = std::path::PathBuf::from(&brew_path).join("bin");
    if !brew_bin.exists() {
        return Err(format!("{} installed but no bin/ directory found", formula));
    }

    std::os::unix::fs::symlink(&brew_bin, &dest_bin)
        .map_err(|e| format!("symlink {} -> {} failed: {}", brew_bin.display(), dest_bin.display(), e))?;

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
