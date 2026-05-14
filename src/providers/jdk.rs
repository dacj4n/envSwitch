use crate::domain::ArchiveFormat;
use crate::infra::download;
use crate::platform::Platform;
use serde::Deserialize;
use std::collections::BTreeSet;

// ── Adoptium (Eclipse Temurin) types ─────────────────────────────────

#[derive(Debug, Deserialize)]
struct AdoptiumAsset {
    version_data: Option<AdoptiumVersionData>,
    binaries: Vec<AdoptiumBinary>,
}

#[derive(Debug, Deserialize)]
struct AdoptiumVersionData { semver: Option<String> }

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    architecture: String,
    os: String,
    package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    name: String,
    link: String,
    #[serde(default)]
    checksum: String,
}

// ── Azul Zulu types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AzulPackage {
    name: String,
    download_url: String,
    #[serde(default)]
    sha256_hash: String,
    #[serde(default)]
    zulu_version: Vec<u32>,
    java_version: Vec<u32>,
}

pub struct JdkProvider;

pub struct JdkAsset {
    pub download_url: String,
    pub checksum: String,
    pub filename: String,
    pub archive_format: ArchiveFormat,
}

impl JdkProvider {
    pub fn fetch_remote_versions() -> Result<Vec<String>, String> {
        let platform = Platform::current();
        let mut all: BTreeSet<String> = BTreeSet::new();

        // ── Azul Zulu (primary — most comprehensive platform coverage) ─
        let mut seen_semver: BTreeSet<String> = BTreeSet::new();
        if let Ok(versions) = fetch_azul_versions(&platform) {
            for v in &versions {
                let semver = v.split('+').next().unwrap_or(v).to_string();
                seen_semver.insert(semver);
                all.insert(v.clone());
            }
        }

        // ── Adoptium (supplement) ────────────────────────────────────
        if let Ok(versions) = fetch_adoptium_versions(&platform) {
            for v in &versions {
                let semver = v.split('+').next().unwrap_or(v).to_string();
                if !seen_semver.contains(&semver) {
                    all.insert(v.clone());
                }
            }
        }

        if all.is_empty() {
            return Err("No JDK versions found for your platform".into());
        }

        let mut sorted: Vec<String> = all.into_iter().collect();
        sorted.sort_by(|a, b| b.cmp(a));
        Ok(sorted)
    }

    pub fn fetch_asset(version: &str) -> Result<JdkAsset, String> {
        let platform = Platform::current();

        // Try Azul first (primary — most comprehensive)
        if let Ok(asset) = fetch_azul_asset(version, &platform) {
            return Ok(asset);
        }

        // Fall back to Adoptium
        fetch_adoptium_asset(version, &platform)
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        // Determine archive type from extension
        let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fmt = if name.ends_with(".zip") {
            ArchiveFormat::Zip
        } else {
            ArchiveFormat::TarGz
        };
        download::extract_archive(archive, dest, &fmt)
    }
}

// ── Adoptium helpers ─────────────────────────────────────────────────

fn fetch_adoptium_versions(platform: &Platform) -> Result<Vec<String>, String> {
    let os = platform.adoptium_os();
    let arch = platform.adoptium_arch();

    let output = std::process::Command::new("curl")
        .args(["-sL", "https://api.adoptium.net/v3/info/available_releases"])
        .output()
        .map_err(|e| format!("curl: {}", e))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON: {}", e))?;
    let majors: Vec<i64> = json["available_releases"].as_array()
        .ok_or("no releases")?
        .iter().filter_map(|v| v.as_i64()).collect();

    let mut versions = Vec::new();
    for major in &majors {
        let url = format!(
            "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?\
             architecture={}&image_type=jdk&jvm_impl=hotspot&os={}&\
             page=0&page_size=5&project=jdk&vendor=eclipse",
            major, arch, os
        );
        let out = std::process::Command::new("curl")
            .args(["-sL", &url])
            .output().map_err(|e| format!("curl: {}", e))?;

        if out.status.success() {
            if let Ok(assets) = serde_json::from_str::<Vec<serde_json::Value>>(&String::from_utf8_lossy(&out.stdout)) {
                for a in &assets {
                    if let Some(ver) = a["version_data"]["semver"].as_str() {
                        versions.push(ver.to_string());
                    }
                }
            }
        }
    }
    Ok(versions)
}

fn fetch_adoptium_asset(version: &str, platform: &Platform) -> Result<JdkAsset, String> {
    let os = platform.adoptium_os();
    let arch = platform.adoptium_arch();
    let major = version.split('.').next().unwrap_or(version);

    let url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?\
         architecture={}&image_type=jdk&jvm_impl=hotspot&os={}&\
         page=0&page_size=10&project=jdk&vendor=eclipse",
        major, arch, os
    );
    let out = std::process::Command::new("curl")
        .args(["-sL", &url])
        .output().map_err(|e| format!("curl: {}", e))?;

    let text = String::from_utf8_lossy(&out.stdout);
    let assets: Vec<AdoptiumAsset> = serde_json::from_str(&text)
        .map_err(|_| format!("No JDK {} from Adoptium for {}", version, platform.display()))?;

    let want_semver = if version.contains('.') { Some(version) } else { None };

    for asset in &assets {
        if let Some(sv) = want_semver {
            if asset.version_data.as_ref().and_then(|vd| vd.semver.as_deref()) != Some(sv) {
                continue;
            }
        }
        for binary in &asset.binaries {
            if binary.architecture == arch && binary.os == os {
                return Ok(JdkAsset {
                    download_url: binary.package.link.clone(),
                    checksum: binary.package.checksum.clone(),
                    filename: binary.package.name.clone(),
                    archive_format: ArchiveFormat::TarGz,
                });
            }
        }
    }
    Err(format!("No JDK {} binary found for {}", version, platform.display()))
}

// ── Azul Zulu helpers ────────────────────────────────────────────────

fn fetch_azul_versions(platform: &Platform) -> Result<Vec<String>, String> {
    let os = azul_os(platform);
    let arch = azul_arch(platform);

    // Query Azul for each major version
    let mut versions = Vec::new();
    for major in &[8, 11, 17, 21, 22, 23, 24, 25, 26] {
        let url = format!(
            "https://api.azul.com/metadata/v1/zulu/packages/\
             ?java_version={}&os={}&arch={}&java_package_type=jdk&page_size=20",
            major, os, arch
        );
        let out = std::process::Command::new("curl")
            .args(["-sL", &url])
            .output().map_err(|e| format!("curl: {}", e))?;

        if out.status.success() {
            if let Ok(pkgs) = serde_json::from_str::<Vec<AzulPackage>>(&String::from_utf8_lossy(&out.stdout)) {
                for p in &pkgs {
                    if p.java_version.len() >= 3 {
                        let ver = if p.zulu_version.len() >= 3 {
                            format!("{}.{}.{}+{}.{}.{}",
                                p.java_version[0], p.java_version[1], p.java_version[2],
                                p.zulu_version[0], p.zulu_version[1], p.zulu_version[2])
                        } else {
                            format!("{}.{}.{}", p.java_version[0], p.java_version[1], p.java_version[2])
                        };
                        versions.push(ver);
                    }
                }
            }
        }
    }
    Ok(versions)
}

fn fetch_azul_asset(version: &str, platform: &Platform) -> Result<JdkAsset, String> {
    let os = azul_os(platform);
    let arch = azul_arch(platform);
    let major = version.split('.').next().unwrap_or(version);

    let url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages/\
         ?java_version={}&os={}&arch={}&java_package_type=jdk&page_size=20",
        major, os, arch
    );
    let out = std::process::Command::new("curl")
        .args(["-sL", &url])
        .output().map_err(|e| format!("curl: {}", e))?;

    let text = String::from_utf8_lossy(&out.stdout);
    let pkgs: Vec<AzulPackage> = serde_json::from_str(&text)
        .map_err(|_| format!("No JDK {} from Azul for {}", version, platform.display()))?;

    let target = version.split('+').next().unwrap_or(version);

    // Collect all matching packages, prefer .tar.gz over .zip (preserves symlinks)
    let mut best: Option<&AzulPackage> = None;
    for p in &pkgs {
        if p.java_version.len() < 3 { continue; }
        let pkg_ver = format!("{}.{}.{}", p.java_version[0], p.java_version[1], p.java_version[2]);
        if pkg_ver == target {
            match best {
                None => best = Some(p),
                Some(ref current) => {
                    // Prefer .tar.gz
                    let current_is_tar = current.name.ends_with(".tar.gz");
                    let this_is_tar = p.name.ends_with(".tar.gz");
                    if this_is_tar && !current_is_tar {
                        best = Some(p);
                    }
                }
            }
        }
    }

    if let Some(p) = best {
        return Ok(JdkAsset {
            download_url: p.download_url.clone(),
            checksum: p.sha256_hash.clone(),
            filename: p.name.clone(),
            archive_format: if p.name.ends_with(".zip") { ArchiveFormat::Zip } else { ArchiveFormat::TarGz },
        });
    }
    Err(format!("No JDK {} from Azul for {}", version, platform.display()))
}

fn azul_os(platform: &Platform) -> &str {
    match platform {
        Platform::MacAarch64 | Platform::MacX64 => "macos",
        Platform::LinuxX64 | Platform::LinuxAarch64 => "linux",
        Platform::WindowsX64 | Platform::WindowsAarch64 => "windows",
    }
}

fn azul_arch(platform: &Platform) -> &str {
    match platform {
        Platform::MacAarch64 | Platform::LinuxAarch64 | Platform::WindowsAarch64 => "aarch64",
        _ => "x64",
    }
}
