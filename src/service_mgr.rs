use crate::domain::RunningService;
use crate::infra::fs;
use chrono::Utc;

/// The result of checking a service's status.
pub struct ServiceStatus {
    pub running: Option<RunningService>,
}

/// Start a service.
pub fn start(module_name: &str, version: &str) -> Result<RunningService, String> {
    let module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    if module.category != crate::domain::ModuleCategory::Service {
        return Err(format!("{} is not a service module.", module_name));
    }

    let install_path = fs::envswitch_home().join("envs").join(module_name).join(version);
    if !install_path.exists() {
        return Err(format!(
            "{} {} is not installed. Run 'envswitch install {} {}' first.",
            module_name, version, module_name, version
        ));
    }

    // Check if already running
    if let Ok(status) = status(module_name) {
        if let Some(svc) = status.running {
            return Err(format!(
                "{} is already running (PID: {}, port: {})",
                module_name, svc.pid, svc.port
            ));
        }
    }

    // Check port
    let port = module.default_port.unwrap_or(3306);
    if let Ok(occupied) = check_port(port) {
        if let Some(info) = occupied {
            return Err(format!(
                "Port {} is already in use by process '{}' (PID: {})",
                port, info.command, info.pid
            ));
        }
    }

    let data_dir = fs::envswitch_home().join("data").join(module_name);
    let _ = std::fs::create_dir_all(&data_dir);

    // Dispatch to adapter
    match module_name {
        "mysql" => {
            crate::providers::mysql::MySqlProvider::init_data_dir(&install_path, &data_dir)?;
            let mut svc = crate::providers::mysql::MySqlProvider::start_service(
                &install_path, &data_dir, port,
            )?;
            svc.version = version.to_string();
            fs::write_pid_file(module_name, svc.pid)
                .map_err(|e| format!("Cannot write PID file: {}", e))?;

            eprintln!("{} {} started (PID: {}, port: {})", module_name, version, svc.pid, svc.port);
            Ok(svc)
        }
        _ => Err(format!("No service adapter for: {}", module_name)),
    }
}

/// Stop a service.
pub fn stop(module_name: &str) -> Result<(), String> {
    let module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    if module.category != crate::domain::ModuleCategory::Service {
        return Err(format!("{} is not a service module.", module_name));
    }

    let check = status(module_name)?;
    if check.running.is_none() {
        eprintln!("{} is not running.", module_name);
        return Ok(());
    }

    let svc = check.running.unwrap();

    match module_name {
        "mysql" => {
            crate::providers::mysql::MySqlProvider::stop_service(svc.pid)?;
        }
        _ => return Err(format!("No service adapter for: {}", module_name)),
    }

    fs::remove_pid_file(module_name).map_err(|e| format!("IO error: {}", e))?;
    eprintln!("{} stopped.", module_name);
    Ok(())
}

/// Check service status.
pub fn status(module_name: &str) -> Result<ServiceStatus, String> {
    let module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    let pid = fs::read_pid_file(module_name);
    match pid {
        Some(p) => {
            // Check if process is actually running
            let running = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(p as i32),
                None, // Signal 0 = check existence
            )
            .is_ok();

            if running {
                let port = module.default_port.unwrap_or(0);
                Ok(ServiceStatus {
                    running: Some(RunningService {
                        module_name: module_name.to_string(),
                        version: "unknown".into(),
                        pid: p,
                        port,
                        started_at: Utc::now(), // Not tracked precisely
                    }),
                })
            } else {
                // Stale PID file
                let _ = fs::remove_pid_file(module_name);
                Ok(ServiceStatus { running: None })
            }
        }
        None => Ok(ServiceStatus { running: None }),
    }
}

/// Check all services.
pub fn status_all() -> Result<Vec<(String, ServiceStatus)>, String> {
    let modules = crate::module_repo::find_by_category(&crate::domain::ModuleCategory::Service);
    let mut results = Vec::new();
    for m in modules {
        match status(&m.name) {
            Ok(s) => results.push((m.name.clone(), s)),
            Err(e) => eprintln!("Warning: {}: {}", m.name, e),
        }
    }
    Ok(results)
}

/// Check if a port is occupied.
pub fn check_port(port: u16) -> Result<Option<PortInfo>, String> {
    // Use lsof to check port
    let output = std::process::Command::new("lsof")
        .args(["-i", &format!(":{}", port), "-t", "-sTCP:LISTEN"])
        .output()
        .map_err(|e| format!("lsof failed: {}", e))?;

    if !output.status.success() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Ok(None);
    }

    if let Ok(pid) = text.lines().next().unwrap_or("").parse::<u32>() {
        let cmd = std::fs::read_to_string(format!("/proc/{}/comm", pid))
            .unwrap_or_else(|_| "unknown".into())
            .trim()
            .to_string();
        Ok(Some(PortInfo { pid, command: cmd }))
    } else {
        Ok(None)
    }
}

pub struct PortInfo {
    pub pid: u32,
    pub command: String,
}

/// Read service logs.
pub fn logs(module_name: &str, lines: usize) -> Result<Vec<String>, String> {
    let data_dir = fs::envswitch_home().join("data").join(module_name);
    match module_name {
        "mysql" => crate::providers::mysql::MySqlProvider::read_logs(&data_dir, lines),
        _ => Err(format!("No logs available for: {}", module_name)),
    }
}
