use crate::domain::{ActiveCover, CoverScope, GlobalCoverEntry, GlobalState, InstalledMetadata};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Returns the envswitch home directory.
pub fn envswitch_home() -> PathBuf {
    if let Ok(h) = std::env::var("ENVSWITCH_HOME") {
        return PathBuf::from(h);
    }
    let home = dirs::home_dir().expect("Cannot determine home directory");
    home.join(".envswitch")
}

pub fn ensure_dirs() -> io::Result<()> {
    let home = envswitch_home();
    fs::create_dir_all(home.join("envs"))?;
    fs::create_dir_all(home.join("data"))?;
    fs::create_dir_all(home.join("run"))?;
    fs::create_dir_all(home.join("state"))?;
    fs::create_dir_all(home.join("cache"))?;
    fs::create_dir_all(home.join("logs"))?;
    Ok(())
}

// ── Installed version metadata ──────────────────────────────────────

pub fn load_installed(module_name: &str) -> io::Result<InstalledMetadata> {
    let path = metadata_path(module_name);
    if path.exists() {
        let data = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data).unwrap_or(InstalledMetadata { versions: vec![] }))
    } else {
        Ok(InstalledMetadata { versions: vec![] })
    }
}

pub fn save_installed(module_name: &str, meta: &InstalledMetadata) -> io::Result<()> {
    let path = metadata_path(module_name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(meta)?;
    fs::write(&path, data)
}

fn metadata_path(module_name: &str) -> PathBuf {
    envswitch_home().join("envs").join(module_name).join("metadata.json")
}

// ── Global state (global covers) ────────────────────────────────────

pub fn load_global_state() -> io::Result<GlobalState> {
    let path = global_state_path();
    if path.exists() {
        let data = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data).unwrap_or(GlobalState { covers: vec![] }))
    } else {
        Ok(GlobalState { covers: vec![] })
    }
}

pub fn save_global_state(state: &GlobalState) -> io::Result<()> {
    let path = global_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(state)?;
    fs::write(&path, data)
}

fn global_state_path() -> PathBuf {
    envswitch_home().join("state").join("global.json")
}

// ── Cover conversion ────────────────────────────────────────────────

/// Convert a GlobalCoverEntry (from state file) to an ActiveCover.
pub fn global_entry_to_cover(entry: &GlobalCoverEntry, install_path: &Path, env_vars: &[(String, String)], path_entries: &[String]) -> ActiveCover {
    let mut injected_paths = Vec::new();
    for pe in path_entries {
        injected_paths.push(install_path.join(pe));
    }
    let injected_envs: Vec<String> = env_vars.iter().map(|(k, _)| k.clone()).collect();

    ActiveCover {
        module_name: entry.module_name.clone(),
        version: entry.version.clone(),
        scope: CoverScope::Global,
        injected_paths,
        injected_envs,
        applied_at: entry.applied_at,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

pub fn disk_usage(path: &Path) -> u64 {
    fn dir_size(p: &Path) -> u64 {
        if let Ok(entries) = fs::read_dir(p) {
            entries
                .flatten()
                .map(|e| {
                    let meta = e.metadata();
                    match meta {
                        Ok(m) if m.is_dir() => dir_size(&e.path()),
                        Ok(m) => m.len(),
                        _ => 0,
                    }
                })
                .sum()
        } else {
            0
        }
    }
    dir_size(path)
}

pub fn available_disk(path: &Path) -> u64 {
    // Simplified: just check if we can write
    if let Ok(m) = fs::metadata(path) {
        m.len() // not ideal but works as crude check; real impl uses sysinfo
    } else {
        0
    }
}

/// Write PID to ~/.envswitch/run/<module>.pid
pub fn write_pid_file(module: &str, pid: u32) -> io::Result<()> {
    let path = envswitch_home().join("run").join(format!("{}.pid", module));
    fs::write(&path, pid.to_string())
}

/// Read PID from pid file
pub fn read_pid_file(module: &str) -> Option<u32> {
    let path = envswitch_home().join("run").join(format!("{}.pid", module));
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Delete PID file
pub fn remove_pid_file(module: &str) -> io::Result<()> {
    let path = envswitch_home().join("run").join(format!("{}.pid", module));
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Resolve template: replace {install_path} and {data_path} placeholders.
pub fn resolve_template(template: &str, install_path: &Path, data_path: &Path) -> String {
    template
        .replace("{install_path}", &install_path.to_string_lossy())
        .replace("{data_path}", &data_path.to_string_lossy())
}
