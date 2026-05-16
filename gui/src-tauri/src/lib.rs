use envswitch::domain::{CoverScope, ModuleCategory};
use envswitch::{install, environment, module_repo, service_mgr, platform};
use serde::Serialize;

#[derive(Serialize)]
struct ModuleInfo {
    name: String,
    display_name: String,
    category: String,
    versions: Vec<String>,
    active_version: Option<String>,
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

        ModuleInfo {
            name: m.name.clone(),
            display_name: m.display_name.clone(),
            category: format!("{:?}", m.category),
            versions: versions.iter().map(|v| v.version.clone()).collect(),
            active_version: active,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
