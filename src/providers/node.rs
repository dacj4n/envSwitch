//! Node.js provider — official nodejs.org downloads, managed entirely by envswitch

use crate::domain::RemoteVersion;
use crate::platform::Platform;
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct NodeVersion {
    version: String,
    files: Vec<String>,
}

pub struct NodeProvider;

pub struct NodeAsset {
    pub download_url: String,
    pub checksum_url: String,
    pub version: String,
}

impl NodeProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = Command::new("curl")
            .args(["-sL", "https://nodejs.org/dist/index.json"])
            .output()
            .map_err(|e| format!("curl: {}", e))?;

        let data: Vec<NodeVersion> = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
            .map_err(|e| format!("Node API: {}", e))?;

        let platform = Platform::current();
        // API files use: osx-arm64-tar, osx-x64-tar, linux-x64, etc.
        let pattern = match platform {
            Platform::MacAarch64 => "osx-arm64-tar",
            Platform::MacX64 => "osx-x64-tar",
            Platform::LinuxX64 => "linux-x64",
            Platform::LinuxAarch64 => "linux-arm64",
            _ => "linux-x64",
        };
        let versions: Vec<RemoteVersion> = data.iter()
            .filter(|v| v.files.iter().any(|f| f.contains(pattern)))
            .map(|v| RemoteVersion { version: v.version.trim_start_matches('v').to_string() })
            .collect();

        if versions.is_empty() {
            return Err("No Node versions found for your platform".into());
        }
        Ok(versions)
    }

    pub fn fetch_asset(version: &str) -> Result<NodeAsset, String> {
        let platform = Platform::current();
        let (os, arch) = node_platform(&platform);
        let filename = format!("node-v{}-{}-{}.tar.gz", version, os, arch);
        let download_url = format!("https://nodejs.org/dist/v{}/{}", version, filename);

        let output = Command::new("curl")
            .args(["-sL", &format!("https://nodejs.org/dist/v{}/SHASUMS256.txt", version)])
            .output()
            .map_err(|_| "failed to fetch SHASUMS".to_string())?;

        let shasums = String::from_utf8_lossy(&output.stdout);

        // Find SHA256 from SHASUMS
        let _sha256 = shasums.lines()
            .find(|l| l.contains(&filename))
            .and_then(|l| l.split_whitespace().next())
            .map(|s| s.to_string())
            .unwrap_or_default();

        Ok(NodeAsset {
            download_url,
            checksum_url: format!("https://nodejs.org/dist/v{}/SHASUMS256.txt", version),
            version: format!("v{}", version),
        })
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        crate::infra::download::extract_archive(archive, dest, &crate::domain::ArchiveFormat::TarGz)
    }
}

pub fn node_platform(platform: &Platform) -> (&str, &str) {
    match platform {
        Platform::MacAarch64 => ("darwin", "arm64"),
        Platform::MacX64 => ("darwin", "x64"),
        Platform::LinuxX64 => ("linux", "x64"),
        Platform::LinuxAarch64 => ("linux", "arm64"),
        _ => ("linux", "x64"),
    }
}
