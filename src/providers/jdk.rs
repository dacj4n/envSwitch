use crate::domain::{ArchiveFormat, ChecksumType};
use crate::infra::download;
use serde::Deserialize;

/// Result from Adoptium assets API.
#[derive(Debug, Deserialize)]
struct AdoptiumAsset {
    binaries: Vec<AdoptiumBinary>,
}

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

/// Adoptium (Eclipse Temurin) JDK provider.
pub struct JdkProvider;

pub struct JdkAsset {
    pub download_url: String,
    pub checksum: String,
    pub filename: String,
}

impl JdkProvider {
    pub fn fetch_remote_versions() -> Result<Vec<String>, String> {
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

        let versions: Vec<String> = json["available_releases"]
            .as_array()
            .ok_or("Unexpected JSON format")?
            .iter()
            .filter_map(|v| v.as_i64().map(|n| n.to_string()))
            .collect();

        if versions.is_empty() {
            return Err("No JDK versions found".into());
        }
        Ok(versions)
    }

    /// Query the Adoptium API for the actual download asset for a version.
    pub fn fetch_asset(version: &str) -> Result<JdkAsset, String> {
        let os = if cfg!(target_os = "macos") { "mac" } else { "linux" };
        let arch = if cfg!(target_arch = "x86_64") { "x64" } else { "aarch64" };

        let api_url = format!(
            "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?\
             architecture={}&image_type=jdk&jvm_impl=hotspot&os={}&\
             page=0&page_size=10&project=jdk&vendor=eclipse",
            version, arch, os
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

        // Find the first matching binary
        for asset in &assets {
            for binary in &asset.binaries {
                if binary.architecture == arch && binary.os == os {
                    return Ok(JdkAsset {
                        download_url: binary.package.link.clone(),
                        checksum: binary.package.checksum.clone(),
                        filename: binary.package.name.clone(),
                    });
                }
            }
        }

        Err(format!(
            "No JDK {} binary found for {}/{}",
            version, os, arch
        ))
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        download::extract_archive(archive, dest, &ArchiveFormat::TarGz)
    }
}
