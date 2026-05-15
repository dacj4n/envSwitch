use crate::domain::RemoteVersion;
use crate::platform::Platform;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct GoFile { filename: String, sha256: String }

#[derive(Debug, Deserialize)]
struct GoVersion { version: String, files: Vec<GoFile> }

pub struct GoProvider;

pub struct GoAsset {
    pub download_url: String,
    pub checksum: String,
    pub version: String,
}

impl GoProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        // Check cache (1 hour TTL)
        let platform = Platform::current();
        if let Some(cached) = read_remote_cache(&platform) {
            return Ok(cached.into_iter().map(|v| RemoteVersion { version: v }).collect());
        }

        let data = fetch_json()?;
        let current = Platform::current();
        let mut versions = Vec::new();

        for v in &data {
            let ver_str = v.version.trim_start_matches("go").to_string();
            // Check if this version has a build for the current platform
            let target_os = current.go_os();
            let target_arch = current.go_arch();
            for f in &v.files {
                if f.filename.contains(target_os) && f.filename.contains(target_arch)
                    && f.filename.ends_with(".tar.gz") {
                    versions.push(RemoteVersion { version: ver_str });
                    break;
                }
            }
        }

        if versions.is_empty() { return Err("No Go versions found".into()); }

        // Cache the version strings
        let version_strs: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
        write_remote_cache(&platform, &version_strs);

        Ok(versions)
    }

    pub fn fetch_asset(version: &str) -> Result<GoAsset, String> {
        let platform = Platform::current();
        let os = platform.go_os();
        let arch = platform.go_arch();
        let data = fetch_json()?;
        let search = format!("go{}", version);

        for v in &data {
            if v.version == search {
                for f in &v.files {
                    if f.filename.contains(os) && f.filename.contains(arch) && f.filename.ends_with(".tar.gz") {
                        return Ok(GoAsset {
                            download_url: format!("https://go.dev/dl/{}", f.filename),
                            checksum: f.sha256.clone(),
                            version: v.version.clone(),
                        });
                    }
                }
            }
        }

        // List what IS available
        let available: Vec<String> = data.iter()
            .filter(|v| v.version == search)
            .flat_map(|v| v.files.iter().map(|f| go_file_platform(&f.filename).unwrap_or_else(|| f.filename.clone())).collect::<Vec<_>>())
            .collect();

        Err(if available.is_empty() {
            format!("No Go {} binary found for {}", version, platform.display())
        } else {
            format!("Go {} is not available for {}.\nAvailable platforms: {}",
                version, platform.display(), available.join(", "))
        })
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        crate::infra::download::extract_archive(archive, dest, &crate::domain::ArchiveFormat::TarGz)
    }
}

/// Extract platform tag from Go filename like "go1.11.1.linux-amd64.tar.gz"
fn go_file_platform(filename: &str) -> Option<String> {
    let name = filename.strip_suffix(".tar.gz")
        .or_else(|| filename.strip_suffix(".zip"))
        .or_else(|| filename.strip_suffix(".msi"))
        .or_else(|| filename.strip_suffix(".pkg"))?;

    let parts: Vec<&str> = name.rsplitn(2, '.').collect();
    let platform_part = parts.first()?;
    // Platform part: "darwin-arm64", "linux-amd64", "windows-amd64", etc.
    let mut segs = platform_part.splitn(2, '-');
    let os = segs.next()?;
    let arch = segs.next().unwrap_or("");

    let os_tag = match os {
        "darwin" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        _ => return None,
    };
    let arch_tag = match arch {
        "amd64" => "x64",
        "arm64" => "ARM64",
        "armv6l" => "ARMv6",
        "386" => "x86",
        "ppc64le" => "ppc64le",
        "s390x" => "s390x",
        _ => arch,
    };
    Some(format!("{} {}", os_tag, arch_tag))
}

fn fetch_json() -> Result<Vec<GoVersion>, String> {
    let output = std::process::Command::new("curl")
        .args(["-sL", "https://go.dev/dl/?mode=json&include=all"])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;
    if !output.status.success() { return Err("Failed to fetch Go versions".into()); }
    let text = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))
}

// ── Cache (1 hour TTL) ───────────────────────────────────────────────

fn cache_path(platform: &Platform) -> std::path::PathBuf {
    crate::infra::fs::envswitch_home()
        .join("cache")
        .join(format!("go_remote_{}.json", platform.go_arch()))
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
