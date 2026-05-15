use crate::domain::{ChecksumType, InstalledVersion};
use crate::infra::{download, fs};
use crate::providers;
use chrono::Utc;
use std::os::unix::fs::PermissionsExt;


/// Install a specific version of a module.
pub fn install(module_name: &str, version: &str, force: bool) -> Result<(), String> {
    let _module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    let dest = fs::envswitch_home().join("envs").join(module_name).join(version);

    eprintln!("Platform: {}", crate::platform::Platform::current().display());

    // Platform compatibility check (before "already installed")
    match module_name {
        "mysql" => {
            // This will fail early if the version/platform combo is unsupported
            let _url = crate::providers::mysql::MySqlProvider::download_url(version)?;
        }
        _ => {}
    }

    // Check if already installed
    if dest.exists() && !force {
        return Err(format!(
            "{} {} is already installed. Use --force to reinstall.",
            module_name, version
        ));
    }

    eprintln!("Downloading {} {} ...", module_name, version);

    // Dispatch to the correct provider
    match module_name {
        "jdk" => {
            eprintln!("Querying Azul Zulu for JDK {}...", version);
            let asset = providers::jdk::JdkProvider::fetch_asset(version)?;
            eprintln!("Downloading {}...", asset.filename);
            let archive = download::download_file(&asset.download_url, module_name, version)?;
            if !asset.checksum.is_empty() {
                eprintln!("Verifying SHA256...");
                download::verify_checksum(&archive, &ChecksumType::Sha256, Some(&asset.checksum))?;
            }
            eprintln!("Extracting...");
            if dest.exists() {
                std::fs::remove_dir_all(&dest).map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            providers::jdk::JdkProvider::install(&archive, &dest)?;
            fix_exec_permissions(&dest)?;
        }
        "go" => {
            eprintln!("Querying go.dev for Go {}...", version);
            let asset = providers::go::GoProvider::fetch_asset(version)?;
            eprintln!("Downloading {}...", asset.version);
            let archive = download::download_file(&asset.download_url, module_name, version)?;
            if !asset.checksum.is_empty() {
                eprintln!("Verifying SHA256...");
                download::verify_checksum(&archive, &ChecksumType::Sha256, Some(&asset.checksum))?;
            }
            eprintln!("Extracting...");
            if dest.exists() {
                std::fs::remove_dir_all(&dest).map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            providers::go::GoProvider::install(&archive, &dest)?;
            // Fix permissions: go.dev tarballs may not preserve exec bits
            fix_exec_permissions(&dest)?;
        }
        "mysql" => {
            let url = providers::mysql::MySqlProvider::download_url(version)?;
            let archive = download::download_file(&url, module_name, version)?;
            eprintln!("Extracting...");
            if dest.exists() {
                std::fs::remove_dir_all(&dest).map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            providers::mysql::MySqlProvider::install(&archive, &dest)?;
        }
        "python" => {
            eprintln!("Python uses MacPorts for installation.");
            if dest.exists() && !force {
                return Err(format!("python {} is already installed. Use --force to reinstall.", version));
            }
            if dest.exists() {
                std::fs::remove_dir_all(&dest).map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            let actual_version = providers::python::PythonProvider::install(version, &dest)?;
            let actual_dest = fs::envswitch_home().join("envs").join(module_name).join(&actual_version);
            if actual_dest != dest {
                let _ = std::fs::remove_dir_all(&actual_dest);
                std::fs::rename(&dest, &actual_dest).map_err(|e| format!("rename: {}", e))?;
                let size = fs::disk_usage(&actual_dest);
                let mut meta = fs::load_installed(module_name).map_err(|e| format!("IO: {}", e))?;
                meta.versions.retain(|v| v.version != actual_version && v.version != version);
                meta.versions.push(InstalledVersion {
                    module_name: module_name.to_string(),
                    version: actual_version.clone(),
                    install_path: actual_dest,
                    installed_at: Utc::now(),
                    size_bytes: size,
                });
                fs::save_installed(module_name, &meta).map_err(|e| format!("IO: {}", e))?;
                eprintln!("python {} installed successfully.", actual_version);
                return Ok(());
            }
        }
        "php" => {
            eprintln!("PHP uses Homebrew for installation.");
            if dest.exists() && !force {
                return Err(format!("php {} is already installed. Use --force to reinstall.", version));
            }
            if dest.exists() {
                std::fs::remove_dir_all(&dest).map_err(|e| format!("Cannot remove old install: {}", e))?;
            }
            let actual_version = providers::php::PhpProvider::install(version, &dest)?;
            // Use actual version from brew for metadata
            let actual_dest = fs::envswitch_home().join("envs").join(module_name).join(&actual_version);
            if actual_dest != dest {
                let _ = std::fs::remove_dir_all(&actual_dest);
                std::fs::rename(&dest, &actual_dest).map_err(|e| format!("rename: {}", e))?;
                // Record metadata with actual version
                let size = fs::disk_usage(&actual_dest);
                let mut meta = fs::load_installed(module_name).map_err(|e| format!("IO: {}", e))?;
                meta.versions.retain(|v| v.version != actual_version && v.version != version);
                meta.versions.push(InstalledVersion {
                    module_name: module_name.to_string(),
                    version: actual_version.clone(),
                    install_path: actual_dest,
                    installed_at: Utc::now(),
                    size_bytes: size,
                });
                fs::save_installed(module_name, &meta).map_err(|e| format!("IO: {}", e))?;
                eprintln!("php {} installed successfully.", actual_version);
                return Ok(());
            }
        }
        _ => return Err(format!("No provider for module: {}", module_name)),
    }

    // Record metadata
    let size = fs::disk_usage(&dest);
    let installed = InstalledVersion {
        module_name: module_name.to_string(),
        version: version.to_string(),
        install_path: dest,
        installed_at: Utc::now(),
        size_bytes: size,
    };

    let mut meta = fs::load_installed(module_name).map_err(|e| format!("IO error: {}", e))?;
    meta.versions.retain(|v| v.version != version);
    meta.versions.push(installed);
    fs::save_installed(module_name, &meta).map_err(|e| format!("IO error: {}", e))?;

    eprintln!("{} {} installed successfully.", module_name, version);
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
                        let _ = std::fs::set_permissions(
                            &path,
                            std::fs::Permissions::from_mode(0o755),
                        );
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
    if covers.iter().any(|c| c.module_name == module_name && c.version == version) {
        return Err(format!(
            "Cannot uninstall {} {}: it is currently covered. Run 'envswitch uncover {}' first.",
            module_name, version, module_name
        ));
    }

    let install_path = fs::envswitch_home().join("envs").join(module_name).join(version);
    if !install_path.exists() {
        return Err(format!("{} {} is not installed.", module_name, version));
    }

    std::fs::remove_dir_all(&install_path)
        .map_err(|e| format!("Failed to remove {}: {}", install_path.display(), e))?;

    // Remove from metadata
    let mut meta = fs::load_installed(module_name).map_err(|e| format!("IO error: {}", e))?;
    meta.versions.retain(|v| v.version != version);
    fs::save_installed(module_name, &meta).map_err(|e| format!("IO error: {}", e))?;

    // Purge data if requested
    if purge {
        let data_dir = fs::envswitch_home().join("data").join(module_name);
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
                let version = entry.file_name().to_string_lossy().to_string();
                if version == "current" || version.starts_with('.') {
                    continue;
                }
                // Find metadata or create stub
                let known = meta.as_ref().and_then(|m| m.versions.iter().find(|v| v.version == version));
                let installed = known.cloned().unwrap_or_else(|| InstalledVersion {
                    module_name: module_name.to_string(),
                    version,
                    install_path: entry.path(),
                    installed_at: chrono::Utc::now(),
                    size_bytes: fs::disk_usage(&entry.path()),
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
pub fn list_all_installed() -> Result<Vec<InstalledVersion>, String> {
    let mut all = Vec::new();
    for m in crate::module_repo::builtin_modules() {
        if let Ok(v) = list_installed(&m.name) {
            all.extend(v);
        }
    }
    Ok(all)
}
