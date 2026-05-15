//! PHP provider — delegates to Homebrew for installation
//! Versions detected via `brew search`, install via `brew install`, symlink for switching.

use crate::domain::RemoteVersion;
use std::collections::BTreeSet;

pub struct PhpProvider;

impl PhpProvider {
    /// Dynamically detect available PHP versions via `brew search`.
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = std::process::Command::new("brew")
            .args(["search", "php"])
            .output()
            .map_err(|_| "Homebrew not found. Install from https://brew.sh".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions = BTreeSet::new();

        for line in text.lines() {
            let line = line.trim();
            // Match: "php@8.3", "shivammathur/php/php@5.6"
            if let Some(ver) = line.split("php@").nth(1) {
                // Extract version: "8.3", "5.6" (strip any suffix like -debug)
                let ver = ver.split_whitespace().next().unwrap_or(ver);
                let ver = ver.split('-').next().unwrap_or(ver);
                if !ver.is_empty() && ver.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                    versions.insert(ver.to_string());
                }
            }
        }

        // Also add versions already installed via Homebrew
        if let Ok(out) = std::process::Command::new("brew").args(["list", "--formula"]).output() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let line = line.trim();
                if let Some(ver) = line.strip_prefix("php@") {
                    let ver = ver.split('-').next().unwrap_or(ver);
                    if !ver.is_empty() && ver.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                        versions.insert(ver.to_string());
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
            return Err("No PHP versions found via Homebrew. Run: brew tap shivammathur/php".into());
        }
        Ok(sorted)
    }

    /// Install PHP via Homebrew and symlink into envswitch.
    pub fn install(version: &str, dest: &std::path::Path) -> Result<(), String> {
        // Determine formula name: check if it's from shivammathur tap or core
        let formula = determine_formula(version)?;

        eprintln!("Installing {} via Homebrew...", formula);
        let status = std::process::Command::new("brew")
            .args(["install", &formula])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("brew install: {}", e))?;

        if !status.success() {
            return Err(format!("brew install {} failed", formula));
        }

        link_brew_to_envswitch(&formula, dest)
    }

    /// Auto-detect already-installed Homebrew PHP and link it.
    pub fn link_existing(version: &str, dest: &std::path::Path) -> Result<(), String> {
        let formula = determine_formula(version)?;
        link_brew_to_envswitch(&formula, dest)
    }
}

fn determine_formula(version: &str) -> Result<String, String> {
    // Check if core formula exists
    let core = format!("php@{}", version);
    let check = std::process::Command::new("brew")
        .args(["--prefix", &core])
        .output()
        .map_err(|_| "Homebrew not found".to_string())?;

    if check.status.success() {
        return Ok(core);
    }

    // Check shivammathur tap
    let tap = format!("shivammathur/php/php@{}", version);
    let check2 = std::process::Command::new("brew")
        .args(["--prefix", &tap])
        .output()
        .map_err(|_| "Homebrew not found".to_string())?;

    if check2.status.success() {
        return Ok(tap);
    }

    // Formula not installed. Try to install from core first.
    // Check brew search to see which tap has it
    let search = std::process::Command::new("brew")
        .args(["search", &format!("php@{}", version)])
        .output()
        .unwrap_or_else(|_| std::process::Output { status: Default::default(), stdout: vec![], stderr: vec![] });

    let text = String::from_utf8_lossy(&search.stdout);
    if text.contains(&format!("shivammathur/php/php@{}", version)) {
        Ok(tap)
    } else {
        Ok(core) // Try core, might work if user taps it
    }
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
