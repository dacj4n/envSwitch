//! PHP provider — php.net API for versions, Homebrew bottles for pre-built binaries
//! On macOS/Linux with Homebrew: downloads pre-built bottles (no compilation needed)
//! Fallback: php.net source tarball

use crate::domain::RemoteVersion;
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
                // Must have at least one downloadable source
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

        // Try Homebrew bottle first (pre-built binary)
        if let Ok(asset) = fetch_homebrew_bottle(version, &platform) {
            return Ok(asset);
        }

        // Fallback: php.net source tarball (needs compilation)
        fetch_phpnet_source(version)
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fmt = if name.ends_with(".tar.gz") || name.ends_with(".bottle.tar.gz") {
            crate::domain::ArchiveFormat::TarGz
        } else if name.ends_with(".tar.xz") {
            crate::domain::ArchiveFormat::TarXz
        } else {
            crate::domain::ArchiveFormat::Zip
        };
        crate::infra::download::extract_archive(archive, dest, &fmt)?;

        // Fix Homebrew bottle paths: binaries are in <dest>/php/<version>/bin/
        // The bottle extracts to a cellar-like structure, find the real bin
        fix_bottle_paths(dest)
    }
}

/// Try to get a Homebrew bottle for PHP.
fn fetch_homebrew_bottle(version: &str, platform: &Platform) -> Result<PhpAsset, String> {
    // Get bottle info via brew
    let output = std::process::Command::new("brew")
        .args(["info", "--json=v2", &format!("php@{}", version)])
        .output()
        .map_err(|_| "brew not available".to_string())?;

    if !output.status.success() {
        // Try main php formula
        let out2 = std::process::Command::new("brew")
            .args(["info", "--json=v2", "php"])
            .output()
            .map_err(|_| "brew not available".to_string())?;

        if !out2.status.success() {
            return Err("Homebrew not available".into());
        }

        let text = String::from_utf8_lossy(&out2.stdout);
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| "brew JSON error".to_string())?;

        let formula = json["formulae"][0].clone();
        let formula_ver = formula["versions"]["stable"].as_str().unwrap_or("");
        if formula_ver != version {
            return Err(format!("Homebrew PHP version mismatch: {} != {}", formula_ver, version));
        }

        return parse_bottle_url(&formula, platform);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "brew JSON error".to_string())?;
    let formula = json["formulae"][0].clone();
    parse_bottle_url(&formula, platform)
}

fn parse_bottle_url(formula: &serde_json::Value, platform: &Platform) -> Result<PhpAsset, String> {
    // Map platform to Homebrew bottle tag
    let bottle_tag = match platform {
        Platform::MacAarch64 => "arm64_tahoe", // macOS 26
        Platform::MacX64 => "sonoma",
        Platform::LinuxX64 => "x86_64_linux",
        Platform::LinuxAarch64 => "arm64_linux",
        _ => return Err("Homebrew bottles not available for this platform".into()),
    };

    let bottles = &formula["bottles"];
    let bottle = &bottles[bottle_tag];

    let url = bottle["url"].as_str().ok_or("no bottle URL")?;
    let sha = bottle["sha256"].as_str().ok_or("no bottle SHA256")?;

    Ok(PhpAsset {
        download_url: url.to_string(),
        checksum: sha.to_string(),
        filename: format!("php-{}.bottle.tar.gz", bottle_tag),
    })
}

/// Homebrew bottles extract to a <name>/<version>/ structure. Find the real bin/.
fn fix_bottle_paths(dest: &std::path::Path) -> Result<(), String> {
    // Homebrew bottles have structure: <dest>/php/<version>/bin/php
    // Check if there's a nested php/<version>/ directory
    let inner = dest.join("php");
    if inner.is_dir() {
        // Find version subdirectory
        if let Ok(entries) = std::fs::read_dir(&inner) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let bin = entry.path().join("bin");
                    if bin.exists() {
                        // Create symlink: <dest>/bin -> inner/<version>/bin
                        let dest_bin = dest.join("bin");
                        let _ = std::fs::remove_file(&dest_bin);
                        let _ = std::fs::remove_dir_all(&dest_bin);
                        std::os::unix::fs::symlink(&bin, &dest_bin)
                            .map_err(|e| format!("symlink: {}", e))?;
                        return Ok(());
                    }
                }
            }
        }
    }
    // No bottle structure, just check if bin/ exists directly
    if dest.join("bin").exists() {
        return Ok(());
    }
    Ok(())
}

/// Fallback: php.net source tarball
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
            });
        }
    }

    Err(format!("No download available for PHP {}", version))
}

fn fetch_releases() -> Result<HashMap<String, PhpRelease>, String> {
    // Query each major version: 3, 4, 5, 7, 8 (6 was never released)
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
