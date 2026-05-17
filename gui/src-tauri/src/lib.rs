use envswitch::domain::{
    CoverScope, InstalledMetadata, InstalledVersion as DomainInstalledVersion, ModuleCategory,
};
use envswitch::infra::oplog::{log_op, OpLevel};
use envswitch::{environment, install, module_repo, platform, providers, service_mgr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::Emitter;

static JOBS: std::sync::LazyLock<Mutex<HashMap<String, JobState>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
static CANCEL_TOKENS: std::sync::LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

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
    phase: String,
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
    phase: String,
    downloaded_bytes: u64,
    total_bytes: u64,
    speed_bytes: u64,
    eta_seconds: u64,
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

    modules
        .iter()
        .map(|m| {
            let versions = install::list_installed(&m.name).unwrap_or_default();
            let active = covers
                .iter()
                .find(|c| c.module_name == m.name)
                .map(|c| c.version.clone());
            let mut deduped: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
            deduped.sort_by(|a, b| b.len().cmp(&a.len()));
            let mut keep: Vec<String> = Vec::new();
            for v in &deduped {
                if !keep.iter().any(|k| k.starts_with(v.as_str()) && k != v) {
                    keep.push(v.clone());
                }
            }
            keep.sort();
            let is_symlinked: Vec<bool> = keep
                .iter()
                .map(|ver| {
                    versions
                        .iter()
                        .find(|iv| &iv.version == ver)
                        .map(|iv| iv.install_path.is_symlink())
                        .unwrap_or(false)
                })
                .collect();
            let source_paths: Vec<String> = keep
                .iter()
                .map(|ver| {
                    versions
                        .iter()
                        .find(|iv| &iv.version == ver)
                        .and_then(|iv| {
                            if iv.install_path.is_symlink() {
                                std::fs::read_link(&iv.install_path)
                                    .ok()
                                    .map(|p| p.display().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                })
                .collect();
            ModuleInfo {
                name: m.name.clone(),
                display_name: m.display_name.clone(),
                category: format!("{:?}", m.category),
                versions: keep,
                active_version: active,
                source_paths,
                is_symlinked,
            }
        })
        .collect()
}

#[tauri::command]
fn cover_module(module: String, version: String, global: bool) -> Result<String, String> {
    let scope = if global {
        CoverScope::Global
    } else {
        CoverScope::Session
    };
    let result = environment::cover(&module, &version, scope);
    match &result {
        Ok(_) => log_op(
            OpLevel::Ok,
            &format!("envswitch cover {} {} — switched", module, version),
        ),
        Err(e) => log_op(
            OpLevel::Error,
            &format!("cover {} {} failed: {}", module, version, e),
        ),
    }
    result
}

#[tauri::command]
fn uncover_module(module: String) -> Result<String, String> {
    let result = environment::uncover(&module);
    if result.is_ok() {
        log_op(OpLevel::Info, &format!("envswitch uncover {}", module));
    }
    result
}

#[tauri::command]
fn uncover_all_modules() -> Result<String, String> {
    let result = environment::uncover_all();
    if result.is_ok() {
        log_op(OpLevel::Info, "envswitch uncover --all");
    }
    result
}

#[tauri::command]
fn get_status() -> Vec<envswitch::domain::ActiveCover> {
    environment::get_status()
}

#[tauri::command]
fn start_service(app: tauri::AppHandle, module: String, version: String) -> String {
    let job_id = next_job_id();
    let m = module.clone();
    let v = version.clone();
    let jid = job_id.clone();

    {
        let mut jobs = JOBS.lock().unwrap();
        jobs.insert(
            job_id.clone(),
            JobState {
                id: job_id.clone(),
                kind: "start".into(),
                module: module.clone(),
                version: version.clone(),
                status: "running".into(),
                progress: 0.0,
                message: "Starting service...".into(),
                phase: "installing".into(),
                logs: vec![],
            },
        );
    }

    let _ = app.emit(
        "job-update",
        JobProgress {
            id: job_id.clone(),
            kind: "start".into(),
            module: module.clone(),
            version: version.clone(),
            status: "running".into(),
            progress: 0.0,
            message: "Starting service...".into(),
            phase: "installing".into(),
            downloaded_bytes: 0,
            total_bytes: 0,
            speed_bytes: 0,
            eta_seconds: 0,
        },
    );

    std::thread::spawn(move || match service_mgr::start(&m, &v) {
        Ok(s) => {
            let msg = format!("{} {} started — PID: {}, Port: {}", m, v, s.pid, s.port);
            log_op(
                OpLevel::Ok,
                &format!(
                    "envswitch start {} {} — pid {} port {}",
                    m, v, s.pid, s.port
                ),
            );
            let mut jobs = JOBS.lock().unwrap();
            if let Some(j) = jobs.get_mut(&jid) {
                j.status = "success".into();
                j.progress = 1.0;
                j.message = msg.clone();
            }
            let _ = app.emit(
                "job-update",
                JobProgress {
                    id: jid,
                    kind: "start".into(),
                    module: m,
                    version: v,
                    status: "success".into(),
                    progress: 1.0,
                    message: msg,
                    phase: "done".into(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    speed_bytes: 0,
                    eta_seconds: 0,
                },
            );
        }
        Err(e) => {
            let mut jobs = JOBS.lock().unwrap();
            if let Some(j) = jobs.get_mut(&jid) {
                j.status = "failed".into();
                j.message = format!("Error: {}", e);
            }
            let _ = app.emit(
                "job-update",
                JobProgress {
                    id: jid,
                    kind: "start".into(),
                    module: m,
                    version: v,
                    status: "failed".into(),
                    progress: 0.0,
                    message: format!("Error: {}", e),
                    phase: "error".into(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    speed_bytes: 0,
                    eta_seconds: 0,
                },
            );
        }
    });
    job_id
}

#[tauri::command]
fn stop_service(app: tauri::AppHandle, module: String) -> String {
    let job_id = next_job_id();
    let m = module.clone();
    let jid = job_id.clone();

    {
        let mut jobs = JOBS.lock().unwrap();
        jobs.insert(
            job_id.clone(),
            JobState {
                id: job_id.clone(),
                kind: "stop".into(),
                module: module.clone(),
                version: String::new(),
                status: "running".into(),
                progress: 0.0,
                message: "Stopping service...".into(),
                phase: "installing".into(),
                logs: vec![],
            },
        );
    }

    let _ = app.emit(
        "job-update",
        JobProgress {
            id: job_id.clone(),
            kind: "stop".into(),
            module: module.clone(),
            version: String::new(),
            status: "running".into(),
            progress: 0.0,
            message: "Stopping service...".into(),
            phase: "installing".into(),
            downloaded_bytes: 0,
            total_bytes: 0,
            speed_bytes: 0,
            eta_seconds: 0,
        },
    );

    std::thread::spawn(move || match service_mgr::stop(&m) {
        Ok(()) => {
            let msg = format!("{} stopped", m);
            log_op(OpLevel::Info, &format!("envswitch stop {} — stopped", m));
            let mut jobs = JOBS.lock().unwrap();
            if let Some(j) = jobs.get_mut(&jid) {
                j.status = "success".into();
                j.progress = 1.0;
                j.message = msg.clone();
            }
            let _ = app.emit(
                "job-update",
                JobProgress {
                    id: jid,
                    kind: "stop".into(),
                    module: m,
                    version: String::new(),
                    status: "success".into(),
                    progress: 1.0,
                    message: msg,
                    phase: "done".into(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    speed_bytes: 0,
                    eta_seconds: 0,
                },
            );
        }
        Err(e) => {
            let mut jobs = JOBS.lock().unwrap();
            if let Some(j) = jobs.get_mut(&jid) {
                j.status = "failed".into();
                j.message = format!("Error: {}", e);
            }
            let _ = app.emit(
                "job-update",
                JobProgress {
                    id: jid,
                    kind: "stop".into(),
                    module: m,
                    version: String::new(),
                    status: "failed".into(),
                    progress: 0.0,
                    message: format!("Error: {}", e),
                    phase: "error".into(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    speed_bytes: 0,
                    eta_seconds: 0,
                },
            );
        }
    });
    job_id
}

#[tauri::command]
fn get_services() -> Vec<ServiceInfo> {
    module_repo::find_by_category(&ModuleCategory::Service)
        .iter()
        .map(|m| {
            let status = service_mgr::status(&m.name)
                .unwrap_or(service_mgr::ServiceStatus { running: None });
            ServiceInfo {
                name: m.name.clone(),
                status: if status.running.is_some() {
                    "Running".into()
                } else {
                    "Stopped".into()
                },
                pid: status.running.as_ref().map(|s| s.pid),
                port: status.running.as_ref().map(|s| s.port),
            }
        })
        .collect()
}

// ── Non-blocking search: spawns thread, returns via event ────────────

fn read_cache(module: &str) -> Option<Vec<String>> {
    let p = platform::Platform::current();
    let arch = p.go_arch();
    let cache_dir = envswitch::infra::fs::envswitch_home().join("cache");
    let cache_file = cache_dir.join(format!("{}_remote_{}.json", module, arch));
    if let Ok(meta) = std::fs::metadata(&cache_file) {
        if let Ok(modified) = meta.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                let ttl = match module {
                    "php" | "python" => 21600,
                    "node" => 43200,
                    _ => 86400,
                };
                if elapsed.as_secs() < ttl {
                    if let Ok(data) = std::fs::read_to_string(&cache_file) {
                        if let Ok(versions) = serde_json::from_str::<Vec<String>>(&data) {
                            return Some(versions);
                        }
                    }
                }
            }
        }
    }
    None
}

fn write_cache(module: &str, versions: &[String]) {
    let p = platform::Platform::current();
    let arch = p.go_arch();
    let cache_dir = envswitch::infra::fs::envswitch_home().join("cache");
    let _ = std::fs::create_dir_all(&cache_dir);
    let cache_file = cache_dir.join(format!("{}_remote_{}.json", module, arch));
    let _ = std::fs::write(
        &cache_file,
        serde_json::to_string(versions).unwrap_or_default(),
    );
}

#[tauri::command]
fn search_versions(app: tauri::AppHandle, module: String) -> Vec<String> {
    // Emit cached immediately if available
    if let Some(cached) = read_cache(&module) {
        let _ = app.emit(
            "search-results",
            serde_json::json!({ "module": module, "versions": cached }),
        );
    }

    // Background refresh
    let m = module.clone();
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let versions: Vec<String> = match m.as_str() {
            "jdk" => providers::jdk::JdkProvider::fetch_remote_versions().unwrap_or_default(),
            "go" => providers::go::GoProvider::fetch_remote_versions()
                .unwrap_or_default()
                .iter()
                .map(|v| v.version.clone())
                .collect(),
            "node" => providers::node::NodeProvider::fetch_remote_versions()
                .unwrap_or_default()
                .iter()
                .map(|v| v.version.clone())
                .collect(),
            "php" => providers::php::PhpProvider::fetch_remote_versions()
                .unwrap_or_default()
                .iter()
                .map(|v| v.version.clone())
                .collect(),
            "python" => providers::python::PythonProvider::fetch_remote_versions()
                .unwrap_or_default()
                .iter()
                .map(|v| v.version.clone())
                .collect(),
            "mysql" => providers::mysql::MySqlProvider::fetch_remote_versions()
                .unwrap_or_default()
                .iter()
                .map(|v| v.version.clone())
                .collect(),
            "pgsql" => providers::postgresql::PostgresqlProvider::fetch_remote_versions()
                .unwrap_or_default()
                .iter()
                .map(|v| v.version.clone())
                .collect(),
            _ => vec![],
        };
        if !versions.is_empty() {
            write_cache(&m, &versions);
        }
        let _ = app_handle.emit(
            "search-results",
            serde_json::json!({ "module": m, "versions": versions }),
        );
    });
    vec![]
}

// ── Non-blocking install: returns job_id, emits progress via events ──

#[tauri::command]
fn install_version(app: tauri::AppHandle, module: String, version: String) -> String {
    let job_id = next_job_id();
    let m = module.clone();
    let v = version.clone();
    let jid = job_id.clone();

    {
        let mut jobs = JOBS.lock().unwrap();
        jobs.insert(
            job_id.clone(),
            JobState {
                id: job_id.clone(),
                kind: "install".into(),
                module: module.clone(),
                version: version.clone(),
                status: "running".into(),
                progress: 0.0,
                message: "Starting...".into(),
                phase: "fetching".into(),
                logs: vec![],
            },
        );
    }
    // Open install progress window
    let url = format!("install/{}", job_id);
    let _ = tauri::WebviewWindowBuilder::new(
        &app,
        &format!("install_{}", job_id),
        tauri::WebviewUrl::App(url.into()),
    )
    .title(format!("Installing {} {}", module, version))
    .inner_size(520.0, 420.0)
    .build();

    let cancel_token = Arc::new(AtomicBool::new(false));
    {
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(job_id.clone(), cancel_token.clone());
    }

    let _ = app.emit(
        "job-update",
        JobProgress {
            id: job_id.clone(),
            kind: "install".into(),
            module: module.clone(),
            version: version.clone(),
            status: "running".into(),
            progress: 0.0,
            message: "Starting install...".into(),
            phase: "fetch".into(),
            downloaded_bytes: 0,
            total_bytes: 0,
            speed_bytes: 0,
            eta_seconds: 0,
        },
    );

    let is_download_module = matches!(module.as_str(), "jdk" | "go" | "node");

    std::thread::spawn(move || {
        let cancelled = || cancel_token.load(Ordering::SeqCst);
        let send = |status: &str,
                    phase: &str,
                    progress: f32,
                    msg: &str,
                    dl: u64,
                    tb: u64,
                    sp: u64,
                    eta: u64| {
            let mut jobs = JOBS.lock().unwrap();
            if let Some(j) = jobs.get_mut(&jid) {
                j.status = status.into();
                j.progress = progress;
                j.message = msg.into();
                j.phase = phase.into();
                j.logs.push(msg.into());
            }
            let _ = app.emit(
                "job-update",
                JobProgress {
                    id: jid.clone(),
                    kind: "install".into(),
                    module: m.clone(),
                    version: v.clone(),
                    status: status.into(),
                    progress,
                    message: msg.into(),
                    phase: phase.into(),
                    downloaded_bytes: dl,
                    total_bytes: tb,
                    speed_bytes: sp,
                    eta_seconds: eta,
                },
            );
        };

        // Kill child processes spawned by the install
        let kill_processes = || {
            // Kill any brew install (formula name varies)
            let _ = std::process::Command::new("pkill")
                .args(["-9", "-f", "brew.*install"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            // Kill curl downloading to .envswitch cache (dest path contains .envswitch)
            let _ = std::process::Command::new("pkill")
                .args(["-9", "-f", "curl.*\\.envswitch"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            // Kill tar extracting to .envswitch paths
            let _ = std::process::Command::new("pkill")
                .args(["-9", "-f", "tar.*\\.envswitch"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        };

        // Clean up: remove install dir, metadata, and homebrew formula
        let cleanup = |mod_name: &str, ver: &str| {
            let home = envswitch::infra::fs::envswitch_home();
            let install_path = home.join("envs").join(mod_name).join(ver);
            if install_path.exists() {
                let _ = std::fs::remove_dir_all(&install_path);
            }
            if let Ok(mut meta) = envswitch::infra::fs::load_installed(mod_name) {
                meta.versions.retain(|iv| iv.version != ver);
                let _ = envswitch::infra::fs::save_installed(mod_name, &meta);
            }
            if matches!(
                mod_name,
                "mysql" | "pgsql" | "php" | "python" | "go" | "node" | "jdk"
            ) {
                let formula = install::brew_formula(mod_name, ver);
                let _ = std::process::Command::new("/opt/homebrew/bin/brew")
                    .args(["uninstall", "--force", "--ignore-dependencies", &formula])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
            }
        };

        // Wait for window to open
        std::thread::sleep(std::time::Duration::from_millis(1000));
        if cancelled() {
            send("cancelled", "done", 0.0, "Cancelled", 0, 0, 0, 0);
            CANCEL_TOKENS.lock().unwrap().remove(&jid);
            return;
        }

        // ── Spawn install in sub-thread with log channel ──
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let (log_tx, log_rx) = std::sync::mpsc::channel();
        let m2 = m.clone();
        let v2 = v.clone();

        // Drain helper: emit any queued log lines as GUI progress
        let drain_logs = |rx: &std::sync::mpsc::Receiver<String>, hb: f32, phase: &str| {
            while let Ok(msg) = rx.try_recv() {
                let lc = msg.to_lowercase();
                let phase = if lc.contains("download") || lc.contains("curl") {
                    "downloading"
                } else if lc.contains("verify") || lc.contains("sha256") {
                    "verifying"
                } else if lc.contains("extract") {
                    "extracting"
                } else if lc.contains("install") || lc.contains("success") {
                    "installing"
                } else if lc.contains("query") || lc.contains("resolve") || lc.contains("platform")
                {
                    "fetching"
                } else {
                    phase
                };
                send("running", phase, hb, &msg, 0, 0, 0, 0);
            }
        };

        if is_download_module {
            // ═══ Staging mode (jdk, go, node): install to tmp → atomic rename ═══
            let home = envswitch::infra::fs::envswitch_home();
            let staging = home.join("tmp").join(format!("{}_{}_{}", m2, v2, jid));
            let _ = std::fs::create_dir_all(&staging);
            let staging2 = staging.clone();
            let token2 = cancel_token.clone();

            std::thread::spawn(move || {
                if token2.load(Ordering::SeqCst) {
                    let _ = result_tx.send((Err("Cancelled".into()), staging2));
                    return;
                }
                let result = install::install_staging(
                    &m2,
                    &v2,
                    &staging2,
                    false,
                    Some(&token2),
                    Some(&log_tx),
                );
                // Drop log_tx so the receiver knows we're done sending logs
                drop(log_tx);
                if token2.load(Ordering::SeqCst) {
                    let _ = result_tx.send((Err("Cancelled".into()), staging2));
                } else {
                    let _ = result_tx.send((result, staging2));
                }
            });

            send("running", "fetching", 0.05, "Starting...", 0, 0, 0, 0);

            let mut hb = 0.18f32;
            let mut cancelling = false;
            let final_staging: std::path::PathBuf;
            loop {
                if cancelled() && !cancelling {
                    cancelling = true;
                    kill_processes();
                    send(
                        "cancelling",
                        "downloading",
                        hb,
                        "Cancelling — cleaning up...",
                        0,
                        0,
                        0,
                        0,
                    );
                }
                // Drain log messages first
                drain_logs(&log_rx, hb, "downloading");
                match result_rx.recv_timeout(std::time::Duration::from_secs(if cancelling {
                    1
                } else {
                    2
                })) {
                    Ok((result, staging_dir)) => {
                        // Drain remaining logs
                        drain_logs(&log_rx, hb, "downloading");
                        final_staging = staging_dir;
                        if cancelling || cancelled() {
                            let _ = std::fs::remove_dir_all(&final_staging);
                            cleanup(&m, &v);
                            send(
                                "cancelled",
                                "done",
                                1.0,
                                "Cancelled — staging removed",
                                0,
                                0,
                                0,
                                0,
                            );
                            break;
                        }
                        match result {
                            Ok(()) => {
                                // Atomic rename: staging → envs/{module}/{version}
                                let final_dest = envswitch::infra::fs::envswitch_home()
                                    .join("envs")
                                    .join(&m)
                                    .join(&v);
                                if final_dest.exists() {
                                    let _ = std::fs::remove_dir_all(&final_dest);
                                }
                                if let Some(parent) = final_dest.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                if let Err(e) = std::fs::rename(&final_staging, &final_dest) {
                                    send(
                                        "failed",
                                        "error",
                                        hb,
                                        &format!("Rename failed: {}", e),
                                        0,
                                        0,
                                        0,
                                        0,
                                    );
                                    break;
                                }
                                // Save metadata
                                let size = envswitch::infra::fs::disk_usage(&final_dest);
                                let installed = DomainInstalledVersion {
                                    module_name: m.clone(),
                                    version: v.clone(),
                                    install_path: final_dest,
                                    installed_at: chrono::Utc::now(),
                                    size_bytes: size,
                                };
                                if let Ok(mut meta) = envswitch::infra::fs::load_installed(&m) {
                                    meta.versions.retain(|iv| iv.version != v);
                                    meta.versions.push(installed);
                                    let _ = envswitch::infra::fs::save_installed(&m, &meta);
                                }
                                // Real log messages were already emitted via drain_logs
                                log_op(OpLevel::Ok, &format!("install {} {} — done", m, v));
                                send(
                                    "success",
                                    "done",
                                    1.0,
                                    &format!("{} {} installed successfully", m, v),
                                    0,
                                    0,
                                    0,
                                    0,
                                );
                            }
                            Err(e) => {
                                let _ = std::fs::remove_dir_all(&final_staging);
                                log_op(
                                    OpLevel::Error,
                                    &format!("install {} {} failed: {}", m, v, e),
                                );
                                send("failed", "error", hb, &format!("Error: {}", e), 0, 0, 0, 0);
                            }
                        }
                        break;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if cancelling {
                            kill_processes();
                            continue;
                        }
                        hb = (hb + 0.03).min(0.65);
                        send("running", "downloading", hb, "Downloading...", 0, 0, 0, 0);
                    }
                    Err(_) => {
                        if cancelling {
                            // Staging dir path unknown (channel broken), fallback cleanup
                            cleanup(&m, &v);
                            send(
                                "cancelled",
                                "done",
                                1.0,
                                "Cancelled — cleaned up",
                                0,
                                0,
                                0,
                                0,
                            );
                        }
                        break;
                    }
                }
            }
        } else {
            // ═══ Brew mode (php, python, mysql, pgsql): direct install + uninstall on cancel ═══
            let token3 = cancel_token.clone();
            std::thread::spawn(move || {
                if token3.load(Ordering::SeqCst) {
                    let _ = result_tx.send((Err("Cancelled".into()), std::path::PathBuf::new()));
                    return;
                }
                let result = install::install_with_log(&m2, &v2, false, &log_tx);
                drop(log_tx);
                if token3.load(Ordering::SeqCst) {
                    let _ = result_tx.send((Err("Cancelled".into()), std::path::PathBuf::new()));
                } else {
                    let _ = result_tx.send((result, std::path::PathBuf::new()));
                }
            });

            send("running", "fetching", 0.05, "Starting...", 0, 0, 0, 0);

            let mut hb = 0.18f32;
            let mut cancelling = false;
            loop {
                if cancelled() && !cancelling {
                    cancelling = true;
                    kill_processes();
                    send(
                        "cancelling",
                        "downloading",
                        hb,
                        "Cancelling — cleaning up...",
                        0,
                        0,
                        0,
                        0,
                    );
                }
                drain_logs(&log_rx, hb, "downloading");
                match result_rx.recv_timeout(std::time::Duration::from_secs(if cancelling {
                    1
                } else {
                    2
                })) {
                    Ok((result, _staging)) => {
                        drain_logs(&log_rx, hb, "downloading");
                        if cancelling || cancelled() {
                            cleanup(&m, &v);
                            send(
                                "cancelled",
                                "done",
                                1.0,
                                "Cancelled — installation cleaned up",
                                0,
                                0,
                                0,
                                0,
                            );
                            break;
                        }
                        match result {
                            Ok(()) => {
                                log_op(OpLevel::Ok, &format!("install {} {} — done", m, v));
                                send(
                                    "success",
                                    "done",
                                    1.0,
                                    &format!("{} {} installed successfully", m, v),
                                    0,
                                    0,
                                    0,
                                    0,
                                );
                            }
                            Err(e) => {
                                log_op(
                                    OpLevel::Error,
                                    &format!("install {} {} failed: {}", m, v, e),
                                );
                                send("failed", "error", hb, &format!("Error: {}", e), 0, 0, 0, 0);
                            }
                        }
                        break;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if cancelling {
                            kill_processes();
                            continue;
                        }
                        hb = (hb + 0.03).min(0.65);
                    }
                    Err(_) => {
                        if cancelling {
                            cleanup(&m, &v);
                            send(
                                "cancelled",
                                "done",
                                1.0,
                                "Cancelled — cleaned up",
                                0,
                                0,
                                0,
                                0,
                            );
                        }
                        break;
                    }
                }
            }
        }
        CANCEL_TOKENS.lock().unwrap().remove(&jid);
    });

    job_id
}

#[tauri::command]
fn get_job_state(job_id: String) -> Option<JobProgress> {
    let jobs = JOBS.lock().unwrap();
    jobs.get(&job_id).map(|j| JobProgress {
        id: j.id.clone(),
        kind: j.kind.clone(),
        module: j.module.clone(),
        version: j.version.clone(),
        status: j.status.clone(),
        progress: j.progress,
        message: j.message.clone(),
        phase: j.phase.clone(),
        downloaded_bytes: 0,
        total_bytes: 0,
        speed_bytes: 0,
        eta_seconds: 0,
    })
}

#[tauri::command]
fn cancel_job(app: tauri::AppHandle, job_id: String) -> Result<String, String> {
    // Set cancellation token so the install thread aborts and cleans up
    if let Some(token) = CANCEL_TOKENS.lock().unwrap().get(&job_id) {
        token.store(true, Ordering::SeqCst);
    }
    let mut jobs = JOBS.lock().unwrap();
    if let Some(j) = jobs.get_mut(&job_id) {
        j.status = "cancelling".into();
        j.message = "Cancelling...".into();
        let _ = app.emit(
            "job-update",
            JobProgress {
                id: job_id.clone(),
                kind: j.kind.clone(),
                module: j.module.clone(),
                version: j.version.clone(),
                status: "cancelling".into(),
                progress: j.progress,
                message: "Cancelling — cleaning up...".into(),
                phase: j.phase.clone(),
                downloaded_bytes: 0,
                total_bytes: 0,
                speed_bytes: 0,
                eta_seconds: 0,
            },
        );
    }
    Ok(format!("Job {} cancelled", job_id))
}

// ── Uninstall (also async via job) ───────────────────────────────────

#[tauri::command]
fn uninstall_version(app: tauri::AppHandle, module: String, version: String) -> String {
    let job_id = next_job_id();
    let m = module.clone();
    let v = version.clone();
    let jid = job_id.clone();
    let _ = app.emit(
        "job-update",
        JobProgress {
            id: jid.clone(),
            kind: "uninstall".into(),
            module: m.clone(),
            version: v.clone(),
            status: "running".into(),
            progress: 0.0,
            message: "Uninstalling...".into(),
            phase: "install".into(),
            downloaded_bytes: 0,
            total_bytes: 0,
            speed_bytes: 0,
            eta_seconds: 0,
        },
    );

    std::thread::spawn(move || {
        let send = |status: &str, msg: &str| {
            let _ = app.emit(
                "job-update",
                JobProgress {
                    id: jid.clone(),
                    kind: "uninstall".into(),
                    module: m.clone(),
                    version: v.clone(),
                    status: status.into(),
                    progress: if status == "success" { 1.0 } else { 0.0 },
                    message: msg.into(),
                    phase: "done".into(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    speed_bytes: 0,
                    eta_seconds: 0,
                },
            );
        };
        match install::uninstall(&m, &v, false) {
            Ok(()) => {
                log_op(OpLevel::Info, &format!("uninstall {} {} — done", m, v));
                send("success", &format!("{} {} uninstalled", m, v));
            }
            Err(e) => {
                log_op(
                    OpLevel::Error,
                    &format!("uninstall {} {} failed: {}", m, v, e),
                );
                send("failed", &format!("Error: {}", e));
            }
        }
    });
    job_id
}

#[tauri::command]
fn link_module(module: String, version: String, path: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&path);
    if !src.exists() {
        return Err(format!("Path not found: {}", path));
    }
    let has_bin =
        src.join("bin").is_dir() || src.join("Contents").join("Home").join("bin").is_dir();
    if !has_bin {
        return Err(format!("No bin/ directory found."));
    }
    let dest = envswitch::infra::fs::envswitch_home()
        .join("envs")
        .join(&module)
        .join(&version);
    let _ = std::fs::create_dir_all(dest.parent().unwrap());
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    std::os::unix::fs::symlink(&src, &dest).map_err(|e| format!("symlink: {}", e))?;
    let meta_path = envswitch::infra::fs::envswitch_home()
        .join("envs")
        .join(&module)
        .join("metadata.json");
    let mut meta: InstalledMetadata = if meta_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&meta_path).unwrap_or_default())
            .unwrap_or(InstalledMetadata { versions: vec![] })
    } else {
        InstalledMetadata { versions: vec![] }
    };
    meta.versions.retain(|v| v.version != version);
    meta.versions.push(DomainInstalledVersion {
        module_name: module.clone(),
        version: version.clone(),
        install_path: dest,
        installed_at: chrono::Utc::now(),
        size_bytes: 0,
    });
    let _ = std::fs::create_dir_all(meta_path.parent().unwrap());
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| format!("write: {}", e))?;
    Ok(format!("{} {} linked", module, version))
}

#[tauri::command]
fn get_platform() -> String {
    platform::Platform::current().display().to_string()
}

#[tauri::command]
fn list_installed_versions(module: String) -> Vec<String> {
    install::list_installed(&module)
        .unwrap_or_default()
        .iter()
        .map(|v| v.version.clone())
        .collect()
}

#[tauri::command]
fn read_service_logs(module: String, version: String, lines: usize) -> Vec<String> {
    let data_dir = envswitch::infra::fs::envswitch_home()
        .join("data")
        .join(&module)
        .join(&version);
    match module.as_str() {
        "mysql" => providers::mysql::MySqlProvider::read_logs(&data_dir, lines).unwrap_or_default(),
        "pgsql" => providers::postgresql::PostgresqlProvider::read_logs(&data_dir, lines)
            .unwrap_or_default(),
        _ => vec!["No log reader for this module".into()],
    }
}

#[tauri::command]
fn get_operation_logs(lines: usize) -> Vec<String> {
    envswitch::infra::oplog::read_ops(lines.max(1).min(5000))
}

#[tauri::command]
fn get_proxy() -> Option<String> {
    envswitch::config::get_proxy()
}

#[tauri::command]
fn set_proxy(proxy: String) {
    envswitch::config::set_proxy(&proxy);
}

#[tauri::command]
fn sync_local() -> Vec<SyncResult> {
    let mut results = Vec::new();
    let home = envswitch::infra::fs::envswitch_home();
    let envs_dir = home.join("envs");

    // Helper: get version from binary output
    fn bin_version(bin: &str, args: &[&str], trim_prefix: &str) -> Option<String> {
        std::process::Command::new(bin)
            .args(args)
            .output()
            .ok()
            .and_then(|o| {
                let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let v = ver
                    .trim_start_matches(trim_prefix)
                    .split_whitespace()
                    .next()?
                    .to_string();
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            })
    }

    // Link a full install directory: envs/{module}/{version} -> src (must have bin/)
    fn link_dir(
        results: &mut Vec<SyncResult>,
        envs_dir: &std::path::Path,
        module: &str,
        version: &str,
        src: &std::path::Path,
        src_label: &str,
    ) {
        let dest = envs_dir.join(module).join(version);
        if !dest.exists() && src.join("bin").exists() {
            let _ = std::fs::create_dir_all(dest.parent().unwrap());
            if std::os::unix::fs::symlink(src, &dest).is_ok() {
                results.push(SyncResult {
                    module: module.into(),
                    version: version.into(),
                    path: src.display().to_string(),
                    source: src_label.into(),
                });
            }
        }
    }

    // Link a single binary: envs/{module}/{version}/bin/{bin_name} -> bin_path
    fn link_bin(
        results: &mut Vec<SyncResult>,
        envs_dir: &std::path::Path,
        module: &str,
        version: &str,
        bin_path: &std::path::Path,
        bin_name: &str,
        src_label: &str,
    ) {
        let dest = envs_dir.join(module).join(version);
        if !dest.exists() {
            let _ = std::fs::create_dir_all(dest.join("bin"));
            std::os::unix::fs::symlink(bin_path, dest.join("bin").join(bin_name)).ok();
            results.push(SyncResult {
                module: module.into(),
                version: version.into(),
                path: bin_path
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                source: src_label.into(),
            });
        }
    }

    // ── System JDK (macOS /Library/Java) ──
    let jvm_dir = std::path::PathBuf::from("/Library/Java/JavaVirtualMachines");
    if let Ok(entries) = std::fs::read_dir(&jvm_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let h = e.path().join("Contents").join("Home");
            if h.join("bin").join("java").exists() {
                let ver = name
                    .trim_start_matches("jdk")
                    .trim_start_matches("jdk-")
                    .trim_end_matches(".jdk")
                    .to_string();
                if !ver.is_empty() {
                    link_dir(&mut results, &envs_dir, "jdk", &ver, &h, "system");
                }
            }
        }
    }

    // ── Homebrew versioned formulas (php@8.3, mysql@8.0, etc.) ──
    let brew_opt = std::path::PathBuf::from("/opt/homebrew/opt");
    for (module, prefix) in &[
        ("php", "php@"),
        ("python", "python@"),
        ("mysql", "mysql@"),
        ("pgsql", "postgresql@"),
    ] {
        if let Ok(entries) = std::fs::read_dir(&brew_opt) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) {
                    let ver = name.strip_prefix(prefix).unwrap_or(&name).to_string();
                    if !ver.is_empty() && e.path().join("bin").exists() {
                        link_dir(&mut results, &envs_dir, module, &ver, &e.path(), "homebrew");
                    }
                }
            }
        }
    }

    // ── Homebrew unversioned packages (go, node) ──
    for (module, brew_name, bin_name, version_args, trim_pfx) in &[
        ("go", "go", "go", &["version"][..], "go version go"),
        ("node", "node", "node", &["--version"][..], "v"),
    ] {
        let opt_dir = brew_opt.join(brew_name);
        let bin = opt_dir.join("bin").join(bin_name);
        if bin.exists() {
            if let Some(ver) = bin_version(&bin.display().to_string(), version_args, trim_pfx) {
                link_dir(&mut results, &envs_dir, module, &ver, &opt_dir, "homebrew");
            }
        }
    }

    // ── Homebrew OpenJDK ──
    if let Ok(entries) = std::fs::read_dir(&brew_opt) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("openjdk") {
                let ver = name
                    .strip_prefix("openjdk@")
                    .unwrap_or_else(|| name.strip_prefix("openjdk").unwrap_or(&name));
                let ver = ver.to_string();
                let java_home = e
                    .path()
                    .join("libexec")
                    .join("openjdk.jdk")
                    .join("Contents")
                    .join("Home");
                let effective = if java_home.join("bin").join("java").exists() {
                    java_home
                } else {
                    e.path()
                };
                if !ver.is_empty() && effective.join("bin").exists() {
                    link_dir(&mut results, &envs_dir, "jdk", &ver, &effective, "homebrew");
                }
            }
        }
    }

    // ── fnm Node versions ──
    let fnm_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/fnm/node-versions");
    if let Ok(entries) = std::fs::read_dir(&fnm_dir) {
        for e in entries.flatten() {
            let inst = e.path().join("installation");
            if inst.join("bin").join("node").exists() {
                link_dir(
                    &mut results,
                    &envs_dir,
                    "node",
                    &e.file_name().to_string_lossy().trim_start_matches('v'),
                    &inst,
                    "fnm",
                );
            }
        }
    }

    // ── nvm Node versions ──
    let nvm_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".nvm/versions/node");
    if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
        for e in entries.flatten() {
            let ver = e.file_name().to_string_lossy().to_string();
            if e.path().join("bin").join("node").exists() && !ver.is_empty() {
                link_dir(
                    &mut results,
                    &envs_dir,
                    "node",
                    ver.trim_start_matches('v'),
                    &e.path(),
                    "nvm",
                );
            }
        }
    }

    // ── System Python ──
    let sys_python = std::path::PathBuf::from("/usr/bin/python3");
    if sys_python.exists() {
        if let Ok(out) = std::process::Command::new(&sys_python)
            .arg("--version")
            .output()
        {
            let ver = String::from_utf8_lossy(&out.stdout)
                .trim()
                .trim_start_matches("Python ")
                .to_string();
            if !ver.is_empty() {
                link_bin(
                    &mut results,
                    &envs_dir,
                    "python",
                    &ver,
                    &sys_python,
                    "python3",
                    "system",
                );
                // Also link as 'python'
                let dest = envs_dir
                    .join("python")
                    .join(&ver)
                    .join("bin")
                    .join("python");
                if !dest.exists() {
                    std::os::unix::fs::symlink("python3", &dest).ok();
                }
            }
        }
    }

    // ── System Go (/usr/local/go/bin/go) ──
    let sys_go = std::path::PathBuf::from("/usr/local/go/bin/go");
    if sys_go.exists() {
        if let Some(ver) = bin_version(&sys_go.display().to_string(), &["version"], "go version go")
        {
            let go_root = std::path::PathBuf::from("/usr/local/go");
            link_dir(&mut results, &envs_dir, "go", &ver, &go_root, "system");
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
            cancel_job,
            get_job_state,
            get_proxy,
            set_proxy,
            list_installed_versions,
            read_service_logs,
            get_operation_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
