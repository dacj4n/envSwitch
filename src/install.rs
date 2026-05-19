use crate::domain::{ChecksumType, InstalledVersion};
use crate::infra::{download, fs};
use crate::providers;
use chrono::Utc;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

/// Install a specific version of a module (production path).
pub fn install(module_name: &str, version: &str, force: bool) -> Result<(), String> {
    let dest = fs::envswitch_home()
        .join("envs")
        .join(module_name)
        .join(version);
    install_to(module_name, version, &dest, force, true, None, None)
}

/// Install with log channel for real-time progress (used by Tauri GUI).
#[allow(dead_code)]
pub fn install_with_log(
    module_name: &str,
    version: &str,
    force: bool,
    log_tx: &Sender<String>,
) -> Result<(), String> {
    let dest = fs::envswitch_home()
        .join("envs")
        .join(module_name)
        .join(version);
    install_to(module_name, version, &dest, force, true, None, Some(log_tx))
}

/// Install to a staging directory (no metadata, no "already installed" check against staging).
/// cancel_token: if set, aborts at checkpoints and returns Err("Cancelled").
/// log_tx: if set, sends real-time log lines (for GUI progress window).
#[allow(dead_code)]
pub fn install_staging(
    module_name: &str,
    version: &str,
    staging_dir: &std::path::Path,
    force: bool,
    cancel_token: Option<&AtomicBool>,
    log_tx: Option<&Sender<String>>,
) -> Result<(), String> {
    let _ = std::fs::create_dir_all(staging_dir);
    install_to(
        module_name,
        version,
        staging_dir,
        force,
        false,
        cancel_token,
        log_tx,
    )
}

fn cancelled(token: Option<&AtomicBool>) -> bool {
    token.map(|t| t.load(Ordering::SeqCst)).unwrap_or(false)
}

fn log_msg(tx: Option<&Sender<String>>, msg: &str) {
    eprintln!("{}", msg);
    if let Some(tx) = tx {
        let _ = tx.send(msg.to_string());
    }
}

fn install_to(
    module_name: &str,
    version: &str,
    dest: &std::path::Path,
    force: bool,
    save_meta: bool,
    cancel_token: Option<&AtomicBool>,
    log_tx: Option<&Sender<String>>,
) -> Result<(), String> {
    let _module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    log_msg(
        log_tx,
        &format!(
            "Platform: {}",
            crate::platform::Platform::current().display()
        ),
    );

    // Platform compatibility check (before "already installed")
    {}

    // Check if already installed (only when not staging)
    if save_meta && dest.exists() && !force {
        return Err(format!(
            "{} {} is already installed. Use --force to reinstall.",
            module_name, version
        ));
    }

    if cancelled(cancel_token) {
        return Err("Cancelled".into());
    }

    log_msg(
        log_tx,
        &format!("Downloading {} {} ...", module_name, version),
    );

    // Helper: pick download_file or download_file_with_log based on log_tx
    let dl = |url: &str| -> Result<std::path::PathBuf, String> {
        if let Some(tx) = log_tx {
            download::download_file_with_log(url, module_name, version, tx)
        } else {
            download::download_file(url, module_name, version)
        }
    };

    // Dispatch to the correct provider
    let mut metadata_version = version.to_string();
    let mut metadata_source = "tarball";

    match module_name {
        "jdk" => {
            log_msg(
                log_tx,
                &format!("Querying Azul Zulu for JDK {}...", version),
            );
            let asset = providers::jdk::JdkProvider::fetch_asset(version)?;
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            log_msg(
                log_tx,
                &format!("Downloading {} from {}", asset.filename, asset.download_url),
            );
            let archive = dl(&asset.download_url)?;
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            if !asset.checksum.is_empty() {
                log_msg(log_tx, "Verifying SHA256...");
                download::verify_checksum(&archive, &ChecksumType::Sha256, Some(&asset.checksum))?;
                log_msg(log_tx, "SHA256 verified OK");
            }
            log_msg(log_tx, "Extracting...");
            if dest.exists() {
                std::fs::remove_dir_all(dest)
                    .map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            providers::jdk::JdkProvider::install(&archive, dest)?;
            fix_exec_permissions(dest)?;
        }
        "go" => {
            log_msg(log_tx, &format!("Querying go.dev for Go {}...", version));
            let asset = providers::go::GoProvider::fetch_asset(version)?;
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            log_msg(
                log_tx,
                &format!("Downloading {} from {}", asset.version, asset.download_url),
            );
            let archive = dl(&asset.download_url)?;
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            if !asset.checksum.is_empty() {
                log_msg(log_tx, "Verifying SHA256...");
                download::verify_checksum(&archive, &ChecksumType::Sha256, Some(&asset.checksum))?;
                log_msg(log_tx, "SHA256 verified OK");
            }
            log_msg(log_tx, "Extracting...");
            if dest.exists() {
                std::fs::remove_dir_all(dest)
                    .map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            providers::go::GoProvider::install(&archive, dest)?;
            // Fix permissions: go.dev tarballs may not preserve exec bits
            fix_exec_permissions(dest)?;
        }
        "node" => {
            let asset = providers::node::NodeProvider::fetch_asset(version)?;
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            log_msg(log_tx, &format!("Downloading from {}", asset.download_url));
            let archive = dl(&asset.download_url)?;
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            if !asset.checksum_url.is_empty() {
                let mut chk_cmd = Command::new("curl");
                crate::config::apply_proxy(&mut chk_cmd);
                let chk_out = chk_cmd
                    .args(["-sL", &asset.checksum_url])
                    .output()
                    .map_err(|e| format!("fetch SHASUMS: {}", e))?;
                let shasums = String::from_utf8_lossy(&chk_out.stdout);
                let platform = crate::platform::Platform::current();
                let (node_os, node_arch) = providers::node::node_platform(&platform);
                let filename = format!("node-v{}-{}-{}.tar.gz", version, node_os, node_arch);
                if let Some(expected) = shasums
                    .lines()
                    .find(|l| l.contains(&filename))
                    .and_then(|l| l.split_whitespace().next())
                {
                    log_msg(log_tx, "Verifying SHA256...");
                    download::verify_checksum(&archive, &ChecksumType::Sha256, Some(expected))?;
                    log_msg(log_tx, "SHA256 verified OK");
                }
            }
            log_msg(log_tx, "Extracting...");
            if dest.exists() {
                std::fs::remove_dir_all(dest)
                    .map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            providers::node::NodeProvider::install(&archive, dest)?;
            fix_exec_permissions(dest)?;
        }
        "pgsql" => {
            metadata_source = "brew";
            log_msg(log_tx, "PostgreSQL uses Homebrew for installation.");
            if dest.exists() && !force {
                return Err(format!(
                    "pgsql {} is already installed. Use --force to reinstall.",
                    version
                ));
            }
            if dest.exists() {
                std::fs::remove_dir_all(dest)
                    .map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            metadata_version =
                providers::postgresql::PostgresqlProvider::install_log(version, dest, log_tx)?;
            log_msg(
                log_tx,
                &format!("pgsql {} installed (brew: {})", version, metadata_version),
            );
        }
        "mysql" => {
            metadata_source = "brew";
            log_msg(log_tx, "MySQL uses Homebrew for installation.");
            if dest.exists() && !force {
                return Err(format!(
                    "mysql {} is already installed. Use --force to reinstall.",
                    version
                ));
            }
            if dest.exists() {
                std::fs::remove_dir_all(dest)
                    .map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            metadata_version = providers::mysql::MySqlProvider::install_log(version, dest, log_tx)?;
            log_msg(
                log_tx,
                &format!("mysql {} installed (brew: {})", version, metadata_version),
            );
        }
        "python" => {
            metadata_source = "brew";
            log_msg(log_tx, "Python uses Homebrew for installation.");
            if dest.exists() && !force {
                return Err(format!(
                    "python {} is already installed. Use --force to reinstall.",
                    version
                ));
            }
            if dest.exists() {
                std::fs::remove_dir_all(dest)
                    .map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            metadata_version =
                providers::python::PythonProvider::install_log(version, dest, log_tx)?;
            log_msg(
                log_tx,
                &format!("python {} installed (brew: {})", version, metadata_version),
            );
        }
        "php" => {
            metadata_source = "brew";
            log_msg(log_tx, "PHP uses Homebrew for installation.");
            if dest.exists() && !force {
                return Err(format!(
                    "php {} is already installed. Use --force to reinstall.",
                    version
                ));
            }
            if dest.exists() {
                std::fs::remove_dir_all(dest)
                    .map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            if cancelled(cancel_token) {
                return Err("Cancelled".into());
            }
            metadata_version = providers::php::PhpProvider::install_log(version, dest, log_tx)?;
            log_msg(
                log_tx,
                &format!("php {} installed (brew: {})", version, metadata_version),
            );
        }
        _ => return Err(format!("No provider for module: {}", module_name)),
    }

    log_msg(
        log_tx,
        &format!("{} {} installed successfully", module_name, version),
    );

    if save_meta {
        // Record metadata
        let size = fs::disk_usage(dest);
        let installed = InstalledVersion {
            module_name: module_name.to_string(),
            version: metadata_version,
            install_path: dest.to_path_buf(),
            installed_at: Utc::now(),
            size_bytes: size,
            source: metadata_source.to_string(),
        };

        let mut meta = fs::load_installed(module_name).map_err(|e| format!("IO error: {}", e))?;
        meta.versions.retain(|v| v.install_path != dest);
        meta.versions.push(installed);
        fs::save_installed(module_name, &meta).map_err(|e| format!("IO error: {}", e))?;
    }
    Ok(())
}

/// Uninstall a specific version of a module.
/// Fix executable permissions on bin/ files after tar extraction.
fn fix_exec_permissions(install_path: &std::path::Path) -> Result<(), String> {
    // Handle macOS .jdk bundle: real binaries are in <name>.jdk/Contents/Home/
    let effective_root = find_jdk_home(install_path);

    for subdir in &["bin", "sbin", "libexec"] {
        let dir = effective_root.join(subdir);
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() || path.is_symlink() {
                        let _ =
                            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Find the effective JDK home: if a .jdk bundle exists, use Contents/Home inside it.
pub fn find_jdk_home(install_path: &std::path::Path) -> std::path::PathBuf {
    // Look for .jdk bundle directories
    if let Ok(entries) = std::fs::read_dir(install_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".jdk") && entry.path().is_dir() {
                let home = entry.path().join("Contents").join("Home");
                if home.exists() {
                    return home;
                }
            }
        }
    }
    // Also check if there's a Contents/Home directly (some JDK layouts)
    let home = install_path.join("Contents").join("Home");
    if home.exists() {
        return home;
    }
    install_path.to_path_buf()
}

pub fn uninstall(module_name: &str, version: &str, purge: bool) -> Result<(), String> {
    let module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    // Check if covered
    let covers = crate::environment::get_status();
    if covers
        .iter()
        .any(|c| c.module_name == module_name && c.version == version)
    {
        return Err(format!(
            "Cannot uninstall {} {}: it is currently covered. Run 'envswitch uncover {}' first.",
            module_name, version, module_name
        ));
    }

    let install_path = fs::envswitch_home()
        .join("envs")
        .join(module_name)
        .join(version);
    if !install_path.exists() {
        return Err(format!("{} {} is not installed.", module_name, version));
    }

    std::fs::remove_dir_all(&install_path)
        .map_err(|e| format!("Failed to remove {}: {}", install_path.display(), e))?;

    // For Homebrew-based modules, also uninstall the formula
    match module_name {
        "mysql" | "pgsql" | "php" | "python" | "go" | "node" | "jdk" => {
            let formula = brew_formula(module_name, version);
            eprintln!("Uninstalling {} via Homebrew...", formula);
            let mut uninstall_cmd = crate::providers::homebrew::brew_cmd();
            let _ = uninstall_cmd
                .args(["uninstall", "--force", "--ignore-dependencies", &formula])
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status();
        }
        _ => {}
    }

    // Remove from metadata
    let mut meta = fs::load_installed(module_name).map_err(|e| format!("IO error: {}", e))?;
    meta.versions.retain(|v| v.version != version);
    fs::save_installed(module_name, &meta).map_err(|e| format!("IO error: {}", e))?;

    // Purge data if requested (per-version data dir)
    if purge {
        let data_dir = fs::envswitch_home()
            .join("data")
            .join(module_name)
            .join(version);
        if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir)
                .map_err(|e| format!("Failed to remove data: {}", e))?;
        }
    }

    eprintln!("{} {} uninstalled.", module_name, version);
    if !purge && module.category == crate::domain::ModuleCategory::Service {
        let data_dir = fs::envswitch_home().join("data").join(module_name);
        eprintln!("Data preserved at {}", data_dir.display());
        eprintln!("Use --purge to also remove data.");
    }
    Ok(())
}

/// List all installed versions for a module by scanning the filesystem.
pub fn list_installed(module_name: &str) -> Result<Vec<InstalledVersion>, String> {
    let envs_dir = fs::envswitch_home().join("envs").join(module_name);
    if !envs_dir.exists() {
        return Ok(Vec::new());
    }

    let meta = fs::load_installed(module_name).ok();
    let mut versions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&envs_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if dir_name == "current" || dir_name.starts_with('.') {
                    continue;
                }
                // Match metadata by install_path (dir may be a short key like "8.0"
                // while metadata.version has the full brew version "8.0.46")
                let known = meta
                    .as_ref()
                    .and_then(|m| m.versions.iter().find(|v| v.install_path == entry.path()));
                let installed = known.cloned().unwrap_or_else(|| InstalledVersion {
                    module_name: module_name.to_string(),
                    version: dir_name,
                    install_path: entry.path(),
                    installed_at: chrono::Utc::now(),
                    size_bytes: fs::disk_usage(&entry.path()),
                    source: "unknown".into(),
                });
                versions.push(installed);
            }
        }
    }
    // Sort by version (newest first, simple string sort)
    versions.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(versions)
}

/// List all installed versions across all modules by scanning the filesystem.
pub fn brew_formula(module_name: &str, version: &str) -> String {
    match module_name {
        "mysql" => {
            if version.starts_with("9.") {
                "mysql".into()
            } else {
                format!(
                    "mysql@{}",
                    version.split('.').take(2).collect::<Vec<_>>().join(".")
                )
            }
        }
        "pgsql" => format!(
            "postgresql@{}",
            version.split('.').take(2).collect::<Vec<_>>().join(".")
        ),
        "php" => format!(
            "php@{}",
            version.split('.').take(2).collect::<Vec<_>>().join(".")
        ),
        "python" => format!("python@{}", version),
        // Unversioned Homebrew formulas
        "go" => "go".into(),
        "node" => "node".into(),
        "jdk" => {
            if version.starts_with("1.8") || version.starts_with("8") {
                "openjdk@8".into()
            } else {
                let major = version.split('.').next().unwrap_or(version);
                format!("openjdk@{}", major)
            }
        }
        _ => format!("{}@{}", module_name, version),
    }
}

pub fn list_all_installed() -> Result<Vec<InstalledVersion>, String> {
    let mut all = Vec::new();
    for m in crate::module_repo::builtin_modules() {
        if let Ok(v) = list_installed(&m.name) {
            all.extend(v);
        }
    }
    Ok(all)
}
