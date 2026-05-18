//! Shared Homebrew helpers for PHP, Python, MySQL, PostgreSQL providers.

/// Locate the brew binary. Checks known install paths on macOS and Linux,
/// plus user-local Linuxbrew. Falls back to searching PATH.
fn brew_path() -> std::path::PathBuf {
    let candidates: &[&str] = &[
        "/opt/homebrew/bin/brew",                     // macOS Apple Silicon
        "/usr/local/bin/brew",                        // macOS Intel
        "/home/linuxbrew/.linuxbrew/bin/brew",        // Linuxbrew (system)
        "/home/pi/.linuxbrew/bin/brew",               // Linuxbrew (Raspberry Pi common)
    ];
    for p in candidates {
        let pb = std::path::PathBuf::from(p);
        if pb.exists() {
            return pb;
        }
    }
    // User-local Linuxbrew (e.g. installed without sudo)
    if let Some(home) = dirs::home_dir() {
        let local = home.join(".linuxbrew/bin/brew");
        if local.exists() {
            return local;
        }
    }
    // Fall back to PATH
    std::path::PathBuf::from("brew")
}

/// Build a brew Command. If we resolved a non-PATH path, also ensure
/// the brew bin directory is in PATH so that brew's own subprocesses
/// (e.g. `brew install` which shells out) don't fail.
pub fn brew_cmd() -> std::process::Command {
    let path = brew_path();
    let mut cmd = std::process::Command::new(&path);
    // If we found brew at a known path, put its directory in PATH
    if let Some(parent) = path.parent() {
        let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
        let current = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{}{}{}", parent.display(), sep, current));
    }
    crate::config::apply_proxy(&mut cmd);
    cmd
}

/// Check that brew is available. On Linux without brew, suggest Linuxbrew.
pub fn check_brew() -> Result<(), String> {
    let ok = brew_cmd()
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());

    if ok {
        return Ok(());
    }

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
#[allow(dead_code)]
pub fn brew_ensure(formula: &str) -> Result<(), String> {
    brew_ensure_log(formula, None)
}

/// brew install with optional log streaming (pipes stdout/stderr to log_tx).
pub fn brew_ensure_log(
    formula: &str,
    log_tx: Option<&std::sync::mpsc::Sender<String>>,
) -> Result<(), String> {
    check_brew()?;
    if brew_installed(formula) {
        let msg = format!("{} already installed, linking...", formula);
        if let Some(tx) = log_tx {
            let _ = tx.send(msg.clone());
        }
        eprintln!("{}", msg);
        return Ok(());
    }
    let msg = format!("brew install {}", formula);
    if let Some(tx) = log_tx {
        let _ = tx.send(msg.clone());
    }
    eprintln!("{}", msg);
    let mut cmd = brew_cmd();
    cmd.args(["install", formula]);
    if let Some(tx) = log_tx {
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| format!("brew spawn: {}", e))?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let tx1 = tx.clone();
        let tx2 = tx.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
            {
                let _ = tx1.send(line);
            }
        });
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                let _ = tx2.send(line);
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
    Ok(())
}

pub fn brew_installed(formula: &str) -> bool {
    brew_cmd()
        .args(["list", "--formula", formula])
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn brew_prefix(formula: &str) -> Result<String, String> {
    let output = brew_cmd()
        .args(["--prefix", formula])
        .output()
        .map_err(|e| format!("brew --prefix: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn brew_version(formula: &str) -> Result<String, String> {
    let output = brew_cmd()
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

pub fn brew_symlink_dir(brew_path: &str, dest: &std::path::Path, dir: &str) -> Result<(), String> {
    let src = std::path::PathBuf::from(brew_path).join(dir);
    if src.exists() {
        let dst = dest.join(dir);
        let _ = std::fs::remove_dir_all(&dst);
        let _ = std::fs::remove_file(&dst);
        std::os::unix::fs::symlink(&src, &dst).map_err(|e| format!("symlink {}: {}", dir, e))?;
    }
    Ok(())
}
