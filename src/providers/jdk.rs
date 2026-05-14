use crate::domain::ArchiveFormat;
use crate::infra::download;
use crate::platform::Platform;
use serde::Deserialize;

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

pub struct JdkProvider;

pub struct JdkAsset {
    pub download_url: String,
    pub checksum: String,
    pub filename: String,
}

impl JdkProvider {
    pub fn fetch_remote_versions() -> Result<Vec<String>, String> {
        let platform = Platform::current();
        let os = platform.adoptium_os();
        let arch = platform.adoptium_arch();

        // First get major versions
        let output = std::process::Command::new("curl")
            .args(["-sL", "https://api.adoptium.net/v3/info/available_releases"])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if !output.status.success() {
            return Err("Failed to fetch JDK versions".into());
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))?;

        let majors: Vec<i64> = json["available_releases"]
            .as_array()
            .ok_or("Unexpected JSON format")?
            .iter()
            .filter_map(|v| v.as_i64())
            .collect();

        if majors.is_empty() {
            return Err("No JDK versions found".into());
        }

        // For each major, fetch assets to get full semver
        eprintln!("Fetching JDK versions...");
        let mut all_versions = Vec::new();
        for major in &majors {
            let api_url = format!(
                "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?\
                 architecture={}&image_type=jdk&jvm_impl=hotspot&os={}&\
                 page=0&page_size=5&project=jdk&vendor=eclipse",
                major, arch, os
            );
            let out = std::process::Command::new("curl")
                .args(["-sL", &api_url])
                .output()
                .map_err(|e| format!("curl failed: {}", e))?;

            if out.status.success() {
                let txt = String::from_utf8_lossy(&out.stdout);
                if let Ok(assets) = serde_json::from_str::<Vec<serde_json::Value>>(&txt) {
                    for asset in &assets {
                        if let Some(ver) = asset["version_data"]["semver"].as_str() {
                            if !all_versions.contains(&ver.to_string()) {
                                all_versions.push(ver.to_string());
                            }
                        }
                    }
                }
            }
        }

        if all_versions.is_empty() {
            return Err("No JDK versions found for your platform".into());
        }
        all_versions.sort_by(|a, b| b.cmp(a));
        Ok(all_versions)
    }

    pub fn fetch_asset(version: &str) -> Result<JdkAsset, String> {
        let platform = Platform::current();
        let os = platform.adoptium_os();
        let arch = platform.adoptium_arch();

        // Extract major version for API query: "21.0.9+10.0.LTS" → "21", "17" → "17"
        let major = version.split('.').next().unwrap_or(version);

        let api_url = format!(
            "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?\
             architecture={}&image_type=jdk&jvm_impl=hotspot&os={}&\
             page=0&page_size=10&project=jdk&vendor=eclipse",
            major, arch, os
        );

        let output = std::process::Command::new("curl")
            .args(["-sL", &api_url])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if !output.status.success() {
            return Err(format!("Failed to fetch JDK {} assets", version));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let assets: Vec<AdoptiumAsset> =
            serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))?;

        // If user specified full semver, match it; otherwise take latest
        let want_semver = if version.contains('.') { Some(version) } else { None };

        for asset in &assets {
            // If looking for specific semver, check match
            if let Some(sv) = want_semver {
                if let Some(ref vd) = asset.version_data {
                    if let Some(ref semver) = vd.semver {
                        if semver != sv { continue; }
                    }
                }
            }
            for binary in &asset.binaries {
                if binary.architecture == arch && binary.os == os {
                    let actual_ver = asset.version_data.as_ref()
                        .and_then(|vd| vd.semver.as_deref())
                        .unwrap_or(version);
                    return Ok(JdkAsset {
                        download_url: binary.package.link.clone(),
                        checksum: binary.package.checksum.clone(),
                        filename: binary.package.name.clone(),
                    });
                }
            }
        }

        Err(format!(
            "No JDK {} binary found for {}",
            version,
            platform.display()
        ))
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        download::extract_archive(archive, dest, &ArchiveFormat::TarGz)
    }
}
