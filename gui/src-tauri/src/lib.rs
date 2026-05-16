use envswitch::domain::{CoverScope, ModuleCategory, InstalledMetadata, InstalledVersion as DomainInstalledVersion};
use envswitch::{install, environment, module_repo, service_mgr, platform, providers};
use serde::Serialize;

#[derive(Serialize)]
struct ModuleInfo {
    name: String,
    display_name: String,
    category: String,
    versions: Vec<String>,
    active_version: Option<String>,
    source_paths: Vec<String>,
    is_symlinked: Vec<bool>,
}

#[derive(Serialize)]
struct ServiceInfo {
    name: String,
    status: String,
    pid: Option<u32>,
    port: Option<u16>,
}

#[tauri::command]
fn list_modules() -> Vec<ModuleInfo> {
    let modules = module_repo::builtin_modules();
    let covers = environment::get_status();

    modules.iter().map(|m| {
        let versions = install::list_installed(&m.name)
            .unwrap_or_default();
        let active = covers.iter()
            .find(|c| c.module_name == m.name)
            .map(|c| c.version.clone());

        // Dedup + check if symlink
        let mut deduped: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
        deduped.sort_by(|a, b| b.len().cmp(&a.len()));
        let mut keep: Vec<String> = Vec::new();
        for v in &deduped {
            if !keep.iter().any(|k| k.starts_with(v.as_str()) && k != v) {
                keep.push(v.clone());
            }
        }
        keep.sort();

        // Check which versions are symlinks (can't uninstall)
        let is_symlinked: Vec<bool> = keep.iter().map(|ver| {
            versions.iter()
                .find(|iv| &iv.version == ver)
                .map(|iv| iv.install_path.is_symlink())
                .unwrap_or(false)
        }).collect();

        // Get source paths for symlinked installs
        let source_paths: Vec<String> = keep.iter().map(|ver| {
            versions.iter()
                .find(|iv| &iv.version == ver)
                .and_then(|iv| {
                    if iv.install_path.is_symlink() {
                        std::fs::read_link(&iv.install_path).ok()
                            .map(|p| p.display().to_string())
                    } else { None }
                })
                .unwrap_or_default()
        }).collect();

        ModuleInfo {
            name: m.name.clone(),
            display_name: m.display_name.clone(),
            category: format!("{:?}", m.category),
            versions: keep,
            active_version: active,
            source_paths,
            is_symlinked,
        }
    }).collect()
}

#[tauri::command]
fn cover_module(module: String, version: String, global: bool) -> Result<String, String> {
    let scope = if global { CoverScope::Global } else { CoverScope::Session };
    environment::cover(&module, &version, scope)
}

#[tauri::command]
fn uncover_module(module: String) -> Result<String, String> {
    environment::uncover(&module)
}

#[tauri::command]
fn uncover_all_modules() -> Result<String, String> {
    environment::uncover_all()
}

#[tauri::command]
fn get_status() -> Vec<envswitch::domain::ActiveCover> {
    environment::get_status()
}

#[tauri::command]
fn start_service(module: String, version: String) -> Result<String, String> {
    service_mgr::start(&module, &version)
        .map(|s| format!("PID: {}, Port: {}", s.pid, s.port))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_service(module: String) -> Result<String, String> {
    service_mgr::stop(&module).map(|()| "stopped".to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_services() -> Vec<ServiceInfo> {
    module_repo::find_by_category(&ModuleCategory::Service).iter().map(|m| {
        let status = service_mgr::status(&m.name).unwrap_or(service_mgr::ServiceStatus { running: None });
        ServiceInfo {
            name: m.name.clone(),
            status: if status.running.is_some() { "Running".into() } else { "Stopped".into() },
            pid: status.running.as_ref().map(|s| s.pid),
            port: status.running.as_ref().map(|s| s.port),
        }
    }).collect()
}

#[tauri::command]
fn link_module(module: String, version: String, path: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&path);
    if !src.exists() {
        return Err(format!("Path not found: {}", path));
    }
    let has_bin = src.join("bin").is_dir()
        || src.join("Contents").join("Home").join("bin").is_dir();
    if !has_bin {
        return Err(format!("No bin/ directory at {}. Expected a software root with bin/ subdirectory.", path));
    }

    let dest = envswitch::infra::fs::envswitch_home().join("envs").join(&module).join(&version);
    let _ = std::fs::create_dir_all(dest.parent().unwrap());
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    std::os::unix::fs::symlink(&src, &dest)
        .map_err(|e| format!("symlink: {}", e))?;

    // Write metadata
    let meta_path = envswitch::infra::fs::envswitch_home().join("envs").join(&module).join("metadata.json");
    let mut meta: InstalledMetadata = if meta_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&meta_path).unwrap_or_default())
            .unwrap_or(InstalledMetadata { versions: vec![] })
    } else {
        InstalledMetadata { versions: vec![] }
    };
    meta.versions.retain(|v| v.version != version);
    meta.versions.push(DomainInstalledVersion {
        module_name: module.clone(), version: version.clone(),
        install_path: dest, installed_at: chrono::Utc::now(), size_bytes: 0,
    });
    let _ = std::fs::create_dir_all(meta_path.parent().unwrap());
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap_or_default())
        .map_err(|e| format!("write metadata: {}", e))?;

    Ok(format!("{} {} linked from {}", module, version, src.display()))
}

#[tauri::command]
fn search_versions(module: String) -> Vec<String> {
    match module.as_str() {
        "jdk" => envswitch::providers::jdk::JdkProvider::fetch_remote_versions().unwrap_or_default(),
        "go" => envswitch::providers::go::GoProvider::fetch_remote_versions()
            .unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
        "node" => envswitch::providers::node::NodeProvider::fetch_remote_versions()
            .unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
        "php" => envswitch::providers::php::PhpProvider::fetch_remote_versions()
            .unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
        "python" => envswitch::providers::python::PythonProvider::fetch_remote_versions()
            .unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
        "mysql" => envswitch::providers::mysql::MySqlProvider::fetch_remote_versions()
            .unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
        "pgsql" => envswitch::providers::postgresql::PostgresqlProvider::fetch_remote_versions()
            .unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
        _ => vec![],
    }
}

#[tauri::command]
fn install_version(module: String, version: String) -> Result<String, String> {
    install::install(&module, &version, false).map(|()| format!("{} {} installed", module, version))
}

#[tauri::command]
fn uninstall_version(module: String, version: String) -> Result<String, String> {
    install::uninstall(&module, &version, false).map(|()| format!("{} {} uninstalled", module, version))
}

#[tauri::command]
fn get_platform() -> String {
    platform::Platform::current().display().to_string()
}

#[derive(Serialize)]
struct SyncResult {
    module: String,
    version: String,
    path: String,
    source: String,
}

#[tauri::command]
fn sync_local() -> Vec<SyncResult> {
    let mut results = Vec::new();
    let home = envswitch::infra::fs::envswitch_home();
    let envs_dir = home.join("envs");

    // Helper: check if version path exists and link it
    let mut link = |module: &str, version: &str, src: std::path::PathBuf, src_label: &str| {
        let dest = envs_dir.join(module).join(version);
        if !dest.exists() && src.join("bin").exists() {
            let _ = std::fs::create_dir_all(dest.parent().unwrap());
            if std::os::unix::fs::symlink(&src, &dest).is_ok() {
                results.push(SyncResult {
                    module: module.into(), version: version.into(),
                    path: src.display().to_string(), source: src_label.into()
                });
            }
        }
    };

    // ── JDK ──────────────────────────────────────────────────────
    let java_home = std::path::PathBuf::from("/Library/Java/JavaVirtualMachines");
    if let Ok(entries) = std::fs::read_dir(&java_home) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let home_dir = e.path().join("Contents").join("Home");
            if home_dir.join("bin").join("java").exists() {
                let ver = name.trim_start_matches("jdk").trim_start_matches("jdk-")
                    .trim_end_matches(".jdk").to_string();
                if !ver.is_empty() { link("jdk", &ver, home_dir, "system"); }
            }
        }
    }

    // ── Homebrew kegs ────────────────────────────────────────────
    let brew_prefix = std::path::PathBuf::from("/opt/homebrew/opt");
    for (module, prefix) in &[("php", "php@"), ("python", "python@"), ("mysql", "mysql@"), ("pgsql", "postgresql@")] {
        if let Ok(entries) = std::fs::read_dir(&brew_prefix) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) {
                    let ver = name.strip_prefix(prefix).unwrap_or(&name).to_string();
                    if !ver.is_empty() && e.path().join("bin").exists() {
                        link(module, &ver, e.path(), "homebrew");
                    }
                }
            }
        }
    }

    // ── fnm Node ─────────────────────────────────────────────────
    let fnm_dir = dirs::home_dir().unwrap_or_default().join(".local/share/fnm/node-versions");
    if let Ok(entries) = std::fs::read_dir(&fnm_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let inst = e.path().join("installation");
            if inst.join("bin").join("node").exists() {
                let ver = name.trim_start_matches('v').to_string();
                link("node", &ver, inst, "fnm");
            }
        }
    }

    // ── System paths ──────────────────────────────────────────────
    // macOS system Python
    let sys_python = std::path::PathBuf::from("/usr/bin/python3");
    if sys_python.exists() {
        if let Ok(out) = std::process::Command::new(&sys_python).arg("--version").output() {
            let ver_str = String::from_utf8_lossy(&out.stdout);
            let ver = ver_str.trim().trim_start_matches("Python ").to_string();
            if !ver.is_empty() {
                let dest = envs_dir.join("python").join(&ver);
                if !dest.exists() {
                    let _ = std::fs::create_dir_all(dest.join("bin"));
                    std::os::unix::fs::symlink(&sys_python, dest.join("bin").join("python3")).ok();
                    std::os::unix::fs::symlink(&sys_python, dest.join("bin").join("python")).ok();
                    // Also symlink pip3 if exists
                    let sys_pip = std::path::PathBuf::from("/usr/bin/pip3");
                    if sys_pip.exists() {
                        std::os::unix::fs::symlink(&sys_pip, dest.join("bin").join("pip3")).ok();
                    }
                    results.push(SyncResult {
                        module: "python".into(), version: ver,
                        path: "/usr/bin".into(), source: "system".into()
                    });
                }
            }
        }
    }

    results
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_modules,
            cover_module,
            uncover_module,
            uncover_all_modules,
            get_status,
            start_service,
            stop_service,
            get_services,
            get_platform,
            sync_local,
            link_module,
            search_versions,
            install_version,
            uninstall_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
