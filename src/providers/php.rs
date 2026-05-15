//! PHP provider — php.net API for versions, shivammathur bottles for pre-built binaries
//! Bottles: ghcr.io/shivammathur/php (pre-built ARM64/x64, no compilation needed)
//! Fallback: php.net source tarball + compile

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
    pub filename: String,
    /// Whether this is a pre-built binary (no compilation needed)
    pub is_prebuilt: bool,
}

impl PhpProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let platform = Platform::current();
        if let Some(cached) = read_cache(&platform) {
            return Ok(cached);
        }

        let releases = fetch_releases()?;
        let mut versions: Vec<RemoteVersion> = releases
            .iter()
            .filter(|(k, v)| {
                if k.contains("alpha") || k.contains("beta") || k.contains("RC") { return false; }
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
        let platform = Platform::current();
        let is_arm64 = matches!(platform,
            Platform::MacAarch64 | Platform::LinuxAarch64 | Platform::WindowsAarch64);
        let is_old = {
            let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();
            parts.first().map_or(true, |&m| m < 7)
                || (parts.first() == Some(&7) && parts.get(1).map_or(true, |&m| m < 4))
        };

        // ARM64 + old PHP (pre-7.4): need shivammathur patched source
        if is_arm64 && is_old {
            if let Ok(asset) = fetch_shivammathur_source(version) {
                return Ok(asset);
            }
        }

        // x64 or new PHP: php.net source compiles natively
        fetch_phpnet_source(version)
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fmt = if name.ends_with(".tar.gz") || name.ends_with(".bottle.tar.gz") {
            ArchiveFormat::TarGz
        } else if name.ends_with(".tar.xz") {
            ArchiveFormat::TarXz
        } else if name.ends_with(".tar.bz2") {
            ArchiveFormat::TarBz2
        } else {
            ArchiveFormat::Zip
        };
        crate::infra::download::extract_archive(archive, dest, &fmt)?;
        fix_bottle_paths(dest)
    }
}

// ── shivammathur patched source ─────────────────────────────────────

/// Fetch ARM64-patched PHP source from shivammathur/homebrew-php formula.
fn fetch_shivammathur_source(version: &str) -> Result<PhpAsset, String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return Err("invalid version format".into());
    }
    let formula = format!("php@{}.{}", parts[0], parts[1]);
    let url = format!(
        "https://raw.githubusercontent.com/shivammathur/homebrew-php/main/Formula/{}.rb",
        formula
    );

    let output = std::process::Command::new("curl")
        .args(["-sL", &url])
        .output()
        .map_err(|_| format!("shivammathur: {} not available", formula))?;

    if !output.status.success() {
        return Err(format!("formula {} not found", formula));
    }

    let text = String::from_utf8_lossy(&output.stdout);

    // Extract source URL from formula: url "https://..."
    let src_url = text
        .lines()
        .find(|l| l.trim().starts_with("url "))
        .and_then(|l| l.split('"').nth(1))
        .ok_or("no source URL in formula")?;

    // Extract SHA256
    let sha256 = text
        .lines()
        .find(|l| l.trim().starts_with("sha256 "))
        .and_then(|l| l.split('"').nth(1))
        .ok_or("no sha256 in formula")?;

    Ok(PhpAsset {
        download_url: src_url.to_string(),
        checksum: sha256.to_string(),
        filename: format!("php-{}.src.tar.gz", version),
        is_prebuilt: false, // still needs compilation, but patched for ARM64
    })
}

/// Find real bin/ after extraction (handles nested Homebrew prefix).
fn fix_bottle_paths(dest: &std::path::Path) -> Result<(), String> {
    // Bottle extracts to: <dest>/php@X.Y/<version>/bin/php
    // We want: <dest>/bin/php
    if dest.join("bin").exists() {
        return Ok(());
    }
    // Search for bin/php inside the extracted tree
    if let Ok(entries) = std::fs::read_dir(dest) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                // Look for version directories inside
                if let Ok(inner) = std::fs::read_dir(entry.path()) {
                    for e in inner.flatten() {
                        let bin = e.path().join("bin");
                        if bin.exists() {
                            let dest_bin = dest.join("bin");
                            let _ = std::fs::remove_file(&dest_bin);
                            let _ = std::fs::remove_dir_all(&dest_bin);
                            std::os::unix::fs::symlink(&bin, &dest_bin)
                                .map_err(|_| "symlink failed".to_string())?;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    Ok(()) // No nested structure found, bin/ might be at root
}

// ── php.net source (fallback) ────────────────────────────────────────

fn fetch_phpnet_source(version: &str) -> Result<PhpAsset, String> {
    let releases = fetch_releases()?;
    let release = releases
        .get(version)
        .ok_or_else(|| format!("PHP {} not found", version))?;

    if let Some(sources) = &release.source {
        if let Some(src) = sources.iter().find(|s| !s.filename.is_empty()) {
            return Ok(PhpAsset {
                download_url: format!("https://www.php.net/distributions/{}", src.filename),
                checksum: src.sha256.clone().unwrap_or_default(),
                filename: src.filename.clone(),
                is_prebuilt: false,
            });
        }
    }

    Err(format!("No download available for PHP {}", version))
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
