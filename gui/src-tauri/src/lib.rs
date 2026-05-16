use envswitch::domain::{CoverScope, ModuleCategory, InstalledMetadata, InstalledVersion as DomainInstalledVersion};
use envswitch::{install, environment, module_repo, service_mgr, platform, providers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Emitter;

static JOBS: std::sync::LazyLock<Mutex<HashMap<String, JobState>>> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn next_job_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("job_{}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobState {
    id: String,
    kind: String,
    module: String,
    version: String,
    status: String, // "running", "success", "failed"
    progress: f32,
    message: String,
    logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct JobProgress {
    id: String,
    kind: String,
    module: String,
    version: String,
    status: String,
    progress: f32,
    message: String,
}

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

#[derive(Serialize)]
struct SyncResult {
    module: String,
    version: String,
    path: String,
    source: String,
}

// ── Tauri Commands ────────────────────────────────────────────────────

#[tauri::command]
fn list_modules() -> Vec<ModuleInfo> {
    let modules = module_repo::builtin_modules();
    let covers = environment::get_status();

    modules.iter().map(|m| {
        let versions = install::list_installed(&m.name).unwrap_or_default();
        let active = covers.iter().find(|c| c.module_name == m.name).map(|c| c.version.clone());
        let mut deduped: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
        deduped.sort_by(|a, b| b.len().cmp(&a.len()));
        let mut keep: Vec<String> = Vec::new();
        for v in &deduped {
            if !keep.iter().any(|k| k.starts_with(v.as_str()) && k != v) { keep.push(v.clone()); }
        }
        keep.sort();
        let is_symlinked: Vec<bool> = keep.iter().map(|ver| {
            versions.iter().find(|iv| &iv.version == ver)
                .map(|iv| iv.install_path.is_symlink()).unwrap_or(false)
        }).collect();
        let source_paths: Vec<String> = keep.iter().map(|ver| {
            versions.iter().find(|iv| &iv.version == ver)
                .and_then(|iv| if iv.install_path.is_symlink() {
                    std::fs::read_link(&iv.install_path).ok().map(|p| p.display().to_string())
                } else { None }).unwrap_or_default()
        }).collect();
        ModuleInfo { name: m.name.clone(), display_name: m.display_name.clone(), category: format!("{:?}", m.category), versions: keep, active_version: active, source_paths, is_symlinked }
    }).collect()
}

#[tauri::command]
fn cover_module(module: String, version: String, global: bool) -> Result<String, String> {
    let scope = if global { CoverScope::Global } else { CoverScope::Session };
    environment::cover(&module, &version, scope)
}

#[tauri::command]
fn uncover_module(module: String) -> Result<String, String> { environment::uncover(&module) }

#[tauri::command]
fn uncover_all_modules() -> Result<String, String> { environment::uncover_all() }

#[tauri::command]
fn get_status() -> Vec<envswitch::domain::ActiveCover> { environment::get_status() }

#[tauri::command]
fn start_service(module: String, version: String) -> Result<String, String> {
    service_mgr::start(&module, &version).map(|s| format!("PID: {}, Port: {}", s.pid, s.port)).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_service(module: String) -> Result<String, String> { service_mgr::stop(&module).map(|()| "stopped".to_string()).map_err(|e| e.to_string()) }

#[tauri::command]
fn get_services() -> Vec<ServiceInfo> {
    module_repo::find_by_category(&ModuleCategory::Service).iter().map(|m| {
        let status = service_mgr::status(&m.name).unwrap_or(service_mgr::ServiceStatus { running: None });
        ServiceInfo { name: m.name.clone(), status: if status.running.is_some() { "Running".into() } else { "Stopped".into() }, pid: status.running.as_ref().map(|s| s.pid), port: status.running.as_ref().map(|s| s.port) }
    }).collect()
}

// ── Non-blocking search: spawns thread, returns via event ────────────

#[tauri::command]
fn search_versions(app: tauri::AppHandle, module: String) -> Vec<String> {
    let m = module.clone();
    // Return cached if available, then spawn fresh search
    std::thread::spawn(move || {
        let versions: Vec<String> = match m.as_str() {
            "jdk" => providers::jdk::JdkProvider::fetch_remote_versions().unwrap_or_default(),
            "go" => providers::go::GoProvider::fetch_remote_versions().unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
            "node" => providers::node::NodeProvider::fetch_remote_versions().unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
            "php" => providers::php::PhpProvider::fetch_remote_versions().unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
            "python" => providers::python::PythonProvider::fetch_remote_versions().unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
            "mysql" => providers::mysql::MySqlProvider::fetch_remote_versions().unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
            "pgsql" => providers::postgresql::PostgresqlProvider::fetch_remote_versions().unwrap_or_default().iter().map(|v| v.version.clone()).collect(),
            _ => vec![],
        };
        let _ = app.emit("search-results", serde_json::json!({ "module": m, "versions": versions }));
    });
    vec![] // Return empty immediately — results come via event
}

// ── Non-blocking install: returns job_id, emits progress via events ──

#[tauri::command]
fn install_version(app: tauri::AppHandle, module: String, version: String) -> String {
    let job_id = next_job_id();
    let m = module.clone(); let v = version.clone(); let jid = job_id.clone();

    {
        let mut jobs = JOBS.lock().unwrap();
        jobs.insert(job_id.clone(), JobState { id: job_id.clone(), kind: "install".into(), module: module.clone(), version: version.clone(), status: "running".into(), progress: 0.0, message: "Starting...".into(), logs: vec![] });
    }
    let _ = app.emit("job-update", JobProgress { id: job_id.clone(), kind: "install".into(), module: module.clone(), version: version.clone(), status: "running".into(), progress: 0.0, message: "Starting install...".into() });

    std::thread::spawn(move || {
        let send = |status: &str, progress: f32, msg: &str| {
            let mut jobs = JOBS.lock().unwrap();
            if let Some(j) = jobs.get_mut(&jid) { j.status = status.into(); j.progress = progress; j.message = msg.into(); j.logs.push(msg.into()); }
            let _ = app.emit("job-update", JobProgress { id: jid.clone(), kind: "install".into(), module: m.clone(), version: v.clone(), status: status.into(), progress, message: msg.into() });
        };
        send("running", 0.1, &format!("Fetching {} {} ...", m, v));
        match install::install(&m, &v, false) {
            Ok(()) => send("success", 1.0, &format!("{} {} installed", m, v)),
            Err(e) => send("failed", 0.0, &format!("Error: {}", e)),
        }
    });

    job_id
}

// ── Uninstall (also async via job) ───────────────────────────────────

#[tauri::command]
fn uninstall_version(app: tauri::AppHandle, module: String, version: String) -> String {
    let job_id = next_job_id();
    let m = module.clone(); let v = version.clone(); let jid = job_id.clone();
    let _ = app.emit("job-update", JobProgress { id: jid.clone(), kind: "uninstall".into(), module: m.clone(), version: v.clone(), status: "running".into(), progress: 0.0, message: "Uninstalling...".into() });

    std::thread::spawn(move || {
        let send = |status: &str, msg: &str| {
            let _ = app.emit("job-update", JobProgress { id: jid.clone(), kind: "uninstall".into(), module: m.clone(), version: v.clone(), status: status.into(), progress: if status == "success" { 1.0 } else { 0.0 }, message: msg.into() });
        };
        match install::uninstall(&m, &v, false) {
            Ok(()) => send("success", &format!("{} {} uninstalled", m, v)),
            Err(e) => send("failed", &format!("Error: {}", e)),
        }
    });
    job_id
}

#[tauri::command]
fn link_module(module: String, version: String, path: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&path);
    if !src.exists() { return Err(format!("Path not found: {}", path)); }
    let has_bin = src.join("bin").is_dir() || src.join("Contents").join("Home").join("bin").is_dir();
    if !has_bin { return Err(format!("No bin/ directory found.")); }
    let dest = envswitch::infra::fs::envswitch_home().join("envs").join(&module).join(&version);
    let _ = std::fs::create_dir_all(dest.parent().unwrap());
    if dest.exists() { let _ = std::fs::remove_dir_all(&dest); }
    std::os::unix::fs::symlink(&src, &dest).map_err(|e| format!("symlink: {}", e))?;
    let meta_path = envswitch::infra::fs::envswitch_home().join("envs").join(&module).join("metadata.json");
    let mut meta: InstalledMetadata = if meta_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&meta_path).unwrap_or_default()).unwrap_or(InstalledMetadata { versions: vec![] })
    } else { InstalledMetadata { versions: vec![] } };
    meta.versions.retain(|v| v.version != version);
    meta.versions.push(DomainInstalledVersion { module_name: module.clone(), version: version.clone(), install_path: dest, installed_at: chrono::Utc::now(), size_bytes: 0 });
    let _ = std::fs::create_dir_all(meta_path.parent().unwrap());
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap_or_default()).map_err(|e| format!("write: {}", e))?;
    Ok(format!("{} {} linked", module, version))
}

#[tauri::command]
fn get_platform() -> String { platform::Platform::current().display().to_string() }

#[tauri::command]
fn sync_local() -> Vec<SyncResult> {
    let mut results = Vec::new();
    let home = envswitch::infra::fs::envswitch_home(); let envs_dir = home.join("envs");
    let mut link = |module: &str, version: &str, src: std::path::PathBuf, src_label: &str| {
        let dest = envs_dir.join(module).join(version);
        if !dest.exists() && src.join("bin").exists() {
            let _ = std::fs::create_dir_all(dest.parent().unwrap());
            if std::os::unix::fs::symlink(&src, &dest).is_ok() {
                results.push(SyncResult { module: module.into(), version: version.into(), path: src.display().to_string(), source: src_label.into() });
            }
        }
    };
    let jvm_dir = std::path::PathBuf::from("/Library/Java/JavaVirtualMachines");
    if let Ok(entries) = std::fs::read_dir(&jvm_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let h = e.path().join("Contents").join("Home");
            if h.join("bin").join("java").exists() {
                let ver = name.trim_start_matches("jdk").trim_start_matches("jdk-").trim_end_matches(".jdk").to_string();
                if !ver.is_empty() { link("jdk", &ver, h, "system"); }
            }
        }
    }
    let brew_prefix = std::path::PathBuf::from("/opt/homebrew/opt");
    for (module, prefix) in &[("php", "php@"), ("python", "python@"), ("mysql", "mysql@"), ("pgsql", "postgresql@")] {
        if let Ok(entries) = std::fs::read_dir(&brew_prefix) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) {
                    let ver = name.strip_prefix(prefix).unwrap_or(&name).to_string();
                    if !ver.is_empty() && e.path().join("bin").exists() { link(module, &ver, e.path(), "homebrew"); }
                }
            }
        }
    }
    let fnm_dir = dirs::home_dir().unwrap_or_default().join(".local/share/fnm/node-versions");
    if let Ok(entries) = std::fs::read_dir(&fnm_dir) {
        for e in entries.flatten() {
            let inst = e.path().join("installation");
            if inst.join("bin").join("node").exists() {
                link("node", e.file_name().to_string_lossy().trim_start_matches('v'), inst, "fnm");
            }
        }
    }
    let sys_python = std::path::PathBuf::from("/usr/bin/python3");
    if sys_python.exists() {
        if let Ok(out) = std::process::Command::new(&sys_python).arg("--version").output() {
            let ver = String::from_utf8_lossy(&out.stdout).trim().trim_start_matches("Python ").to_string();
            if !ver.is_empty() {
                let dest = envs_dir.join("python").join(&ver);
                if !dest.exists() {
                    let _ = std::fs::create_dir_all(dest.join("bin"));
                    std::os::unix::fs::symlink(&sys_python, dest.join("bin").join("python3")).ok();
                    std::os::unix::fs::symlink(&sys_python, dest.join("bin").join("python")).ok();
                    results.push(SyncResult { module: "python".into(), version: ver, path: "/usr/bin".into(), source: "system".into() });
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
                app.handle().plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_modules, cover_module, uncover_module, uncover_all_modules, get_status,
            start_service, stop_service, get_services, get_platform,
            sync_local, link_module, search_versions, install_version, uninstall_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
