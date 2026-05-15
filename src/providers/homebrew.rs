//! Shared Homebrew helpers for PHP, Python, MySQL, PostgreSQL providers.

/// Check that brew is available. On Linux without brew, suggest Linuxbrew.
pub fn check_brew() -> Result<(), String> {
    let ok = std::process::Command::new("brew")
        .arg("--version")
        .output()
        .map_or(false, |o| o.status.success());

    if ok { return Ok(()); }

    let msg = if cfg!(target_os = "linux") {
        "\
Homebrew is required for this module. Install Linuxbrew:

  /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"

Then add to your shell:

  eval \"$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)\"

Then retry."
    } else {
        "\
Homebrew is required for this module. Install from:

  https://brew.sh"
    };
    Err(msg.to_string())
}

/// Run brew install only if formula is not already installed.
pub fn brew_ensure(formula: &str) -> Result<(), String> {
    check_brew()?;
    if brew_installed(formula) {
        eprintln!("{} already installed, linking...", formula);
        return Ok(());
    }
    eprintln!("Installing {} via Homebrew...", formula);
    let status = std::process::Command::new("brew").args(["install", formula])
        .stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit())
        .status().map_err(|e| format!("brew: {}", e))?;
    if !status.success() { eprintln!("brew link had conflicts (ignored)"); }
    Ok(())
}

pub fn brew_installed(formula: &str) -> bool {
    // brew list --formula <name> succeeds only if installed
    std::process::Command::new("brew")
        .args(["list", "--formula", formula])
        .output()
        .map_or(false, |o| o.status.success())
}

pub fn brew_prefix(formula: &str) -> Result<String, String> {
    let output = std::process::Command::new("brew").args(["--prefix", formula]).output()
        .map_err(|e| format!("brew --prefix: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn brew_version(formula: &str) -> Result<String, String> {
    let output = std::process::Command::new("brew").args(["info", "--json=v2", formula]).output()
        .map_err(|e| format!("brew info: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|_| "brew info parse error".to_string())?;
    json["formulae"][0]["versions"]["stable"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "version not found".to_string())
}

pub fn brew_symlink_dir(brew_path: &str, dest: &std::path::Path, dir: &str) -> Result<(), String> {
    let src = std::path::PathBuf::from(brew_path).join(dir);
    if src.exists() {
        let dst = dest.join(dir);
        let _ = std::fs::remove_dir_all(&dst);
        let _ = std::fs::remove_file(&dst);
        std::os::unix::fs::symlink(&src, &dst)
            .map_err(|e| format!("symlink {}: {}", dir, e))?;
    }
    Ok(())
}
