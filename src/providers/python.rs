//! Python provider — MacPorts pre-built archives (no compilation needed)

use crate::domain::RemoteVersion;
use crate::platform::Platform;

pub struct PythonProvider;

impl PythonProvider {
    pub fn fetch_remote_versions() -> Result<Vec<RemoteVersion>, String> {
        let output = std::process::Command::new("port")
            .args(["search", "python"])
            .output()
            .map_err(|_| "MacPorts not found. Install from https://www.macports.org".to_string())?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut versions: Vec<RemoteVersion> = text
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                // Match: "python312 @3.12.13 (lang)"
                if let Some(rest) = line.strip_prefix("python3") {
                    let num_str = rest.split_whitespace().next().unwrap_or("");
                    // "12" from "12" or "12" from "12 @3.12.13"
                    if num_str.len() >= 2 && num_str.chars().all(|c| c.is_ascii_digit()) {
                        // python312 → "3.12", python310 → "3.10"
                        let minor = num_str;
                        return Some(format!("3.{}", minor));
                    }
                }
                None
            })
            .map(|v| RemoteVersion { version: v })
            .collect();

        versions.sort_by(|a, b| b.version.cmp(&a.version));
        versions.dedup_by(|a, b| a.version == b.version);

        if versions.is_empty() {
            return Err("No Python versions found via MacPorts".into());
        }
        Ok(versions)
    }

    pub fn install(version: &str, dest: &std::path::Path) -> Result<String, String> {
        // Build port name: "3.12" -> "python312"
        let short = version.replace('.', "");
        let portname = format!("python{}", short);

        // Get archive URL from MacPorts packages server
        let platform = Platform::current();
        let darwin_ver = darwin_major_version()?;
        let arch = match platform {
            Platform::MacAarch64 | Platform::LinuxAarch64 | Platform::WindowsAarch64 => "arm64",
            _ => "x86_64",
        };

        // Query port for exact version
        let info = std::process::Command::new("port")
            .args(["info", "--line", "--version", &portname])
            .output()
            .map_err(|_| "MacPorts not found".to_string())?;

        let info_text = String::from_utf8_lossy(&info.stdout).trim().to_string();
        let full_version = info_text.split_whitespace().last().unwrap_or(version);

        // Build archive URL: python312-3.12.13_0+lto+optimizations.darwin_25.arm64.tbz2
        // We need to know the revision and variants. Use a glob approach or port fetch.
        let archive_name = format!(
            "{}-{}_0+lto+optimizations.darwin_{}.{}.tbz2",
            portname, full_version, darwin_ver, arch
        );
        let archive_url = format!(
            "https://packages.macports.org/{}/{}",
            portname, archive_name
        );

        // Download
        let cache_dir = crate::infra::fs::envswitch_home().join("cache").join("python");
        let _ = std::fs::create_dir_all(&cache_dir);
        let dest_file = cache_dir.join(&archive_name);

        if !dest_file.exists() {
            eprintln!("Downloading {}...", archive_name);
            let status = std::process::Command::new("curl")
                .args(["-sL", "-o", &dest_file.to_string_lossy(), &archive_url])
                .status()
                .map_err(|e| format!("download: {}", e))?;

            if !status.success() {
                let _ = std::fs::remove_file(&dest_file);
                return Err(format!("Python {} not available for {}. Try another version.", version, platform.display()));
            }
        }

        // Extract tbz2: MacPorts archives are bzip2-compressed tar
        eprintln!("Extracting...");
        let _ = std::fs::create_dir_all(dest);
        let status = std::process::Command::new("tar")
            .args(["-xjf", &dest_file.to_string_lossy(), "-C", &dest.to_string_lossy()])
            .status()
            .map_err(|e| format!("extract: {}", e))?;

        if !status.success() {
            return Err("Failed to extract Python archive".into());
        }

        // MacPorts installs to /opt/local/, mirror that structure
        // Find bin/python3 in the extracted tree
        let bin_target = find_python_bin(dest)?;

        // Symlink binaries to envswitch bin/
        let dest_bin = dest.join("bin");
        let _ = std::fs::create_dir_all(&dest_bin);
        for name in &["python3", "python", "pip3", "pip"] {
            let src = bin_target.join(name);
            if src.exists() {
                let target = dest_bin.join(name);
                let _ = std::fs::remove_file(&target);
                std::os::unix::fs::symlink(&src, &target)
                    .map_err(|e| format!("symlink {}: {}", name, e))?;
            }
        }

        eprintln!("Python {} installed", full_version);
        Ok(full_version.to_string())
    }
}

fn darwin_major_version() -> Result<u32, String> {
    let output = std::process::Command::new("uname")
        .arg("-r")
        .output()
        .map_err(|e| format!("uname: {}", e))?;
    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
    ver.split('.').next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "Cannot determine Darwin version".to_string())
}

/// Find bin/ directory in MacPorts-extracted tree.
fn find_python_bin(dest: &std::path::Path) -> Result<std::path::PathBuf, String> {
    // MacPorts archive extracts to: opt/local/bin/python3
    let opt_local_bin = dest.join("opt").join("local").join("bin");
    if opt_local_bin.join("python3").exists() {
        return Ok(opt_local_bin);
    }
    Err(format!("Python binary not found in extracted archive at {}", dest.display()))
}
