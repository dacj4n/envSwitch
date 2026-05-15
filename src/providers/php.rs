//! PHP provider — php.net API for version listing and source downloads
//! ARM64: only shows PHP >= 7.4 (older versions don't compile natively)
//! x64: all versions available

use crate::domain::{ArchiveFormat, RemoteVersion};
use crate::platform::Platform;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct PhpRelease {
    source: Option<Vec<PhpSource>>,
}

#[derive(Debug, Deserialize)]
struct PhpSource {
    #[serde(default)]
    #[allow(dead_code)]
    filename: String,
    #[serde(default)]
    sha256: Option<String>,
}

pub struct PhpProvider;

pub struct PhpAsset {
    pub download_url: String,
    pub checksum: String,
}

impl PhpProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let platform = Platform::current();
        if let Some(cached) = read_cache(&platform) {
            return Ok(cached);
        }

        let is_arm64 = matches!(platform,
            Platform::MacAarch64 | Platform::LinuxAarch64 | Platform::WindowsAarch64);

        let releases = fetch_releases()?;
        let mut versions: Vec<RemoteVersion> = releases
            .iter()
            .filter(|(k, v)| {
                if k.contains("alpha") || k.contains("beta") || k.contains("RC") { return false; }
                if is_arm64 {
                    if let Some(major) = k.split('.').next().and_then(|s| s.parse::<u32>().ok()) {
                        if major < 7 { return false; }
                        if major == 7 {
                            if let Some(minor) = k.split('.').nth(1).and_then(|s| s.parse::<u32>().ok()) {
                                if minor < 4 { return false; }
                            }
                        }
                    }
                }
                v.source.as_ref().map_or(false, |s| s.iter().any(|src| !src.filename.is_empty()))
            })
            .map(|(k, _)| RemoteVersion { version: k.clone() })
            .collect();
        versions.sort_by(|a, b| b.version.cmp(&a.version));

        if versions.is_empty() {
            return Err("No PHP versions found".into());
        }
        write_cache(&platform, &versions);
        Ok(versions)
    }

    pub fn fetch_asset(version: &str) -> Result<PhpAsset, String> {
        let releases = fetch_releases()?;
        let release = releases
            .get(version)
            .ok_or_else(|| format!("PHP {} not found", version))?;

        if let Some(sources) = &release.source {
            if let Some(src) = sources.iter().find(|s| !s.filename.is_empty()) {
                return Ok(PhpAsset {
                    download_url: format!("https://www.php.net/distributions/{}", src.filename),
                    checksum: src.sha256.clone().unwrap_or_default(),
                });
            }
        }

        Err(format!("No download available for PHP {}", version))
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fmt = if name.ends_with(".tar.gz") {
            ArchiveFormat::TarGz
        } else if name.ends_with(".tar.xz") {
            ArchiveFormat::TarXz
        } else if name.ends_with(".tar.bz2") {
            ArchiveFormat::TarBz2
        } else {
            ArchiveFormat::Zip
        };
        crate::infra::download::extract_archive(archive, dest, &fmt)
    }
}

// ── php.net API ──────────────────────────────────────────────────────

fn fetch_releases() -> Result<HashMap<String, PhpRelease>, String> {
    let mut all = HashMap::new();
    for major in &["3", "4", "5", "7", "8"] {
        let url = format!(
            "https://www.php.net/releases/index.php?json&version={}&max=200",
            major
        );
        let output = std::process::Command::new("curl")
            .args(["-sL", &url])
            .output()
            .map_err(|e| format!("curl: {}", e))?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Ok(releases) = serde_json::from_str::<HashMap<String, PhpRelease>>(&text) {
                all.extend(releases);
            }
        }
    }
    if all.is_empty() {
        return Err("Failed to fetch PHP releases".into());
    }
    Ok(all)
}

// ── Cache ────────────────────────────────────────────────────────────

fn cache_path() -> std::path::PathBuf {
    crate::infra::fs::envswitch_home().join("cache").join("php_remote.json")
}

fn read_cache(_platform: &Platform) -> Option<Vec<RemoteVersion>> {
    let path = cache_path();
    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed.as_secs() < 3600 {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(strings) = serde_json::from_str::<Vec<String>>(&data) {
                            return Some(strings.into_iter().map(|v| RemoteVersion { version: v }).collect());
                        }
                    }
                }
            }
        }
    }
    None
}

fn write_cache(_platform: &Platform, versions: &[RemoteVersion]) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let strings: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
    if let Ok(data) = serde_json::to_string(&strings) {
        let _ = std::fs::write(&path, data);
    }
}
