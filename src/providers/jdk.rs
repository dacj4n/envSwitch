//! JDK provider — Azul Zulu Metadata API (single-call, no auth needed)
//! Docs: https://docs.azul.com/core/install/metadata-api
//! GET /metadata/v1/zulu/packages/?os=macos&arch=arm&java_package_type=jdk&availability_types=CA&page_size=1000

use crate::domain::ArchiveFormat;
use crate::infra::download;
use crate::platform::Platform;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
struct AzulPackage {
    name: String,
    download_url: String,
    java_version: Vec<u32>,
    #[serde(default)]
    latest: bool,
}

pub struct JdkProvider;

pub struct JdkAsset {
    pub download_url: String,
    pub checksum: String,
    pub filename: String,
}

impl JdkProvider {
    /// Fetch all available JDK versions for the current platform in ONE API call.
    pub fn fetch_remote_versions() -> Result<Vec<String>, String> {
        let platform = Platform::current();

        // Check cache (1 hour TTL)
        if let Some(cached) = read_remote_cache(&platform) {
            return Ok(cached);
        }

        let pkgs = fetch_azul_packages(&platform, None)?;

        let mut all: BTreeSet<String> = BTreeSet::new();
        for p in &pkgs {
            if p.java_version.len() >= 3 {
                let ver = format!("{}.{}.{}", p.java_version[0], p.java_version[1], p.java_version[2]);
                all.insert(ver);
            }
        }

        let mut sorted: Vec<String> = all.into_iter().collect();
        sorted.sort_by(|a, b| {
            let va: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
            let vb: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
            vb.cmp(&va) // newest first
        });

        // Cache for 1 hour
        write_remote_cache(&platform, &sorted);

        if sorted.is_empty() {
            return Err("No JDK versions found for your platform".into());
        }
        Ok(sorted)
    }

    pub fn fetch_asset(version: &str) -> Result<JdkAsset, String> {
        let platform = Platform::current();
        let pkgs = fetch_azul_packages(&platform, None)?;

        let target = version.split('+').next().unwrap_or(version);
        let parts: Vec<u32> = target.split('.').filter_map(|s| s.parse().ok()).collect();

        // Find matching package: same java_version and latest=true
        let mut best: Option<&AzulPackage> = None;
        for p in &pkgs {
            if p.java_version.len() < 3 { continue; }
            if p.java_version[0] as u32 != parts.first().copied().unwrap_or(0) { continue; }
            let pkg_ver = format!("{}.{}.{}", p.java_version[0], p.java_version[1], p.java_version[2]);
            if pkg_ver == target {
                match best {
                    None => best = Some(p),
                    Some(ref current) => {
                        // Prefer .tar.gz over .zip, and latest=true
                        let cur_score = (if current.name.ends_with(".tar.gz") { 2 } else { 1 })
                            + (if current.latest { 1 } else { 0 });
                        let this_score = (if p.name.ends_with(".tar.gz") { 2 } else { 1 })
                            + (if p.latest { 1 } else { 0 });
                        if this_score > cur_score { best = Some(p); }
                    }
                }
            }
        }

        if let Some(p) = best {
            return Ok(JdkAsset {
                download_url: p.download_url.clone(),
                checksum: String::new(), // Azul API doesn't provide checksums
                filename: p.name.clone(),
            });
        }

        Err(format!("No JDK {} found for {}", version, platform.display()))
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fmt = if name.ends_with(".zip") { ArchiveFormat::Zip } else { ArchiveFormat::TarGz };
        download::extract_archive(archive, dest, &fmt)
    }
}

/// Fetch packages from Azul API. If `java_version` is None, returns ALL versions.
fn fetch_azul_packages(platform: &Platform, java_version: Option<u32>) -> Result<Vec<AzulPackage>, String> {
    let os = match platform {
        Platform::MacAarch64 | Platform::MacX64 => "macos",
        Platform::LinuxX64 | Platform::LinuxAarch64 => "linux",
        Platform::WindowsX64 | Platform::WindowsAarch64 => "windows",
    };
    let arch = match platform {
        Platform::MacAarch64 | Platform::LinuxAarch64 | Platform::WindowsAarch64 => "arm",
        _ => "x86",
    };

    let mut url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages/\
         ?os={}&arch={}&java_package_type=jdk&availability_types=CA&page_size=1000",
        os, arch
    );
    if let Some(jv) = java_version {
        url.push_str(&format!("&java_version={}", jv));
    }

    let mut cmd = std::process::Command::new("curl");
    crate::config::apply_proxy(&mut cmd);
    let out = cmd.args(["-sL", "--connect-timeout", "10", &url])
        .output()
        .map_err(|e| format!("curl: {}", e))?;

    if !out.status.success() {
        return Err("Failed to fetch JDK versions from Azul".into());
    }

    let text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&text).map_err(|e| format!("Azul API error: {}", e))
}

// ── Cache (1 hour TTL) ───────────────────────────────────────────────

fn cache_path(platform: &Platform) -> std::path::PathBuf {
    crate::infra::fs::envswitch_home()
        .join("cache")
        .join(format!("jdk_remote_{}.json", platform.go_arch()))
}

fn read_remote_cache(platform: &Platform) -> Option<Vec<String>> {
    let path = cache_path(platform);
    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed.as_secs() < 3600 {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(versions) = serde_json::from_str(&data) {
                            return Some(versions);
                        }
                    }
                }
            }
        }
    }
    None
}

fn write_remote_cache(platform: &Platform, versions: &[String]) {
    let path = cache_path(platform);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string(versions) {
        let _ = std::fs::write(&path, data);
    }
}
