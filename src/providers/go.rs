use serde::Deserialize;

/// Go download file entry from go.dev JSON API.
#[derive(Debug, Deserialize)]
struct GoFile {
    filename: String,
    sha256: String,
}

/// Go version entry from go.dev JSON API.
#[derive(Debug, Deserialize)]
struct GoVersion {
    version: String,
    stable: bool,
    files: Vec<GoFile>,
}

/// Go provider using go.dev download API.
pub struct GoProvider;

pub struct GoAsset {
    pub download_url: String,
    pub checksum: String,
    pub version: String,
}

impl GoProvider {
    pub fn fetch_remote_versions() -> Result<Vec<String>, String> {
        let data = fetch_json()?;
        let versions: Vec<String> = data
            .iter()
            .filter(|v| v.stable)
            .map(|v| v.version.trim_start_matches("go").to_string())
            .collect();
        if versions.is_empty() {
            return Err("No Go versions found".into());
        }
        Ok(versions)
    }

    /// Fetch the download URL and checksum for a specific version.
    pub fn fetch_asset(version: &str) -> Result<GoAsset, String> {
        let os = if cfg!(target_os = "macos") { "darwin" } else { "linux" };
        let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };

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

        Err(format!("No Go {} binary found for {}/{}", version, os, arch))
    }

    pub fn install(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        crate::infra::download::extract_archive(archive, dest, &crate::domain::ArchiveFormat::TarGz)
    }
}

fn fetch_json() -> Result<Vec<GoVersion>, String> {
    let output = std::process::Command::new("curl")
        .args(["-sL", "https://go.dev/dl/?mode=json"])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if !output.status.success() {
        return Err("Failed to fetch Go versions".into());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))
}
