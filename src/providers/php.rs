//! PHP provider — official php.net release API
//! GET https://www.php.net/releases/index.php?json&version=8&max=50

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
    filename: String,
    sha256: Option<String>,
}

pub struct PhpProvider;

pub struct PhpAsset {
    pub download_url: String,
    pub checksum: String,
    pub filename: String,
}

impl PhpProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let platform = Platform::current();
        if let Some(cached) = read_cache(&platform) {
            return Ok(cached);
        }

        let releases = fetch_releases()?;
        let mut versions: Vec<RemoteVersion> = releases
            .keys()
            .filter(|k| !k.contains("alpha") && !k.contains("beta") && !k.contains("RC"))
            .cloned()
            .map(|v| RemoteVersion { version: v })
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
            if let Some(src) = sources.first() {
                return Ok(PhpAsset {
                    download_url: format!(
                        "https://www.php.net/distributions/{}",
                        src.filename
                    ),
                    checksum: src.sha256.clone().unwrap_or_default(),
                    filename: src.filename.clone(),
                });
            }
        }

        Err(format!("No download available for PHP {}", version))
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        crate::infra::download::extract_archive(archive, dest, &ArchiveFormat::TarGz)
    }
}

fn fetch_releases() -> Result<HashMap<String, PhpRelease>, String> {
    let output = std::process::Command::new("curl")
        .args([
            "-sL",
            "https://www.php.net/releases/index.php?json&version=8&max=50",
        ])
        .output()
        .map_err(|e| format!("curl: {}", e))?;

    if !output.status.success() {
        return Err("Failed to fetch PHP releases".into());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&text).map_err(|e| format!("PHP API error: {}", e))
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
    // Store just version strings
    let strings: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
    if let Ok(data) = serde_json::to_string(&strings) {
        let _ = std::fs::write(&path, data);
    }
}
