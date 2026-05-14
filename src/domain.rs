use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Enums ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModuleCategory {
    Sdk,
    Service,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveFormat {
    TarGz,
    TarXz,
    Zip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChecksumType {
    Sha256,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoverScope {
    Session,
    Global,
}

impl CoverScope {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "session" => Some(CoverScope::Session),
            "global" => Some(CoverScope::Global),
            _ => None,
        }
    }
}

// ── Value Objects ───────────────────────────────────────────────────

/// Environment change delta — the core output of cover/uncover.
/// The CLI renders this to shell script; it never mutates env directly.
#[derive(Debug, Clone, Default)]
pub struct EnvDelta {
    /// Variables to export: KEY=VALUE
    pub exports: HashMap<String, String>,
    /// Variables to unset
    pub unset_vars: Vec<String>,
    /// Paths to prepend to PATH
    pub path_prepend: Vec<PathBuf>,
    /// Paths to remove from PATH
    pub path_remove: Vec<PathBuf>,
}

// ── Entities ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub display_name: String,
    pub category: ModuleCategory,
    /// (VAR_NAME, value_template) — e.g. ("JAVA_HOME", "{install_path}")
    pub env_vars: Vec<(String, String)>,
    /// Relative subdirs to add to PATH, e.g. ["bin"]
    pub path_entries: Vec<String>,
    /// Default port for services
    pub default_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub module_name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub installed_at: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Active cover record. Stores what WE injected (not a snapshot of previous state).
/// This enables multi-module uncover without interference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveCover {
    pub module_name: String,
    pub version: String,
    pub scope: CoverScope,
    /// Paths THIS cover injected into PATH
    pub injected_paths: Vec<PathBuf>,
    /// Env var names THIS cover set
    pub injected_envs: Vec<String>,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningService {
    pub module_name: String,
    pub version: String,
    pub pid: u32,
    pub port: u16,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub dependencies: Vec<(String, String)>,
}

// ── Traits ──────────────────────────────────────────────────────────

/// Strategy for downloading and installing a runtime.
pub trait RuntimeProvider {
    fn fetch_remote_versions(&self) -> Result<Vec<String>, String>;
    fn download_url(&self, version: &str) -> String;
    fn checksum_url(&self, version: &str) -> Option<String>;
    fn archive_format(&self) -> ArchiveFormat;
    fn install(&self, archive: &Path, dest: &Path) -> Result<(), String>;
}

/// Strategy for managing a service (MySQL, etc.) lifecycle.
pub trait ServiceAdapter {
    fn init_data_dir(&self, install_path: &Path, data_dir: &Path) -> Result<(), String>;
    fn start(&self, install_path: &Path, data_dir: &Path, port: u16)
        -> Result<RunningService, String>;
    fn stop(&self, service: &RunningService) -> Result<(), String>;
    fn is_running(&self, data_dir: &Path) -> bool;
    fn read_logs(&self, data_dir: &Path, lines: usize) -> Result<Vec<String>, String>;
}

// ── State persistence types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalState {
    pub covers: Vec<GlobalCoverEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCoverEntry {
    pub module_name: String,
    pub version: String,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledMetadata {
    pub versions: Vec<InstalledVersion>,
}

/// A remote version with platform availability info.
#[derive(Debug, Clone)]
pub struct RemoteVersion {
    pub version: String,
    /// Platform tags this version is available for, e.g. ["macOS ARM64", "macOS x64"]
    pub platforms: Vec<String>,
    /// Whether this version is available for the current platform
    pub current_platform: bool,
}
