use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::domain::ChecksumType;
use crate::infra::fs::envswitch_home;

/// Download a file from url to cache_dir, return path to downloaded file.
/// If log_tx is provided, curl's stderr progress bar is streamed in real-time.
pub fn download_file(url: &str, module: &str, version: &str) -> Result<PathBuf, String> {
    download_file_inner(url, module, version, None)
}

pub fn download_file_with_log(
    url: &str,
    module: &str,
    version: &str,
    log_tx: &Sender<String>,
) -> Result<PathBuf, String> {
    download_file_inner(url, module, version, Some(log_tx))
}

fn download_file_inner(
    url: &str,
    module: &str,
    version: &str,
    log_tx: Option<&Sender<String>>,
) -> Result<PathBuf, String> {
    let cache_dir = envswitch_home().join("cache").join(module);
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;

    let filename = url.split('/').next_back().unwrap_or("archive");
    let dest = cache_dir.join(format!("{}-{}", version, filename));

    if dest.exists() {
        return Ok(dest);
    }

    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "-L",
        "--connect-timeout",
        "15",
        "--max-time",
        "0",
        "-o",
        &dest.to_string_lossy(),
        "-#",
        url,
    ]);
    crate::config::apply_proxy(&mut cmd);

    if let Some(tx) = log_tx {
        // Pipe stderr to capture the progress bar in real-time
        cmd.stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| format!("curl: {}", e))?;
        let mut stderr = child.stderr.take().unwrap();

        let mut buf = [0u8; 1];
        let mut line = String::new();
        loop {
            match stderr.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let ch = buf[0] as char;
                    if ch == '\r' || ch == '\n' {
                        let trimmed = line.trim().to_string();
                        if !trimmed.is_empty() {
                            let _ = tx.send(trimmed);
                        }
                        line.clear();
                    } else {
                        line.push(ch);
                    }
                }
                Err(_) => break,
            }
        }
        // Flush any remaining
        let trimmed = line.trim().to_string();
        if !trimmed.is_empty() {
            let _ = tx.send(trimmed);
        }

        let status = child.wait().map_err(|e| format!("curl wait: {}", e))?;
        if !status.success() {
            let _ = fs::remove_file(&dest);
            return Err(format!(
                "Download failed with exit code: {:?}",
                status.code()
            ));
        }
    } else {
        let status = cmd
            .status()
            .map_err(|e| format!("curl not found: {}. Please install curl.", e))?;
        if !status.success() {
            let _ = fs::remove_file(&dest);
            return Err(format!(
                "Download failed with exit code: {:?}",
                status.code()
            ));
        }
    }

    Ok(dest)
}

/// Verify SHA256 checksum if applicable.
pub fn verify_checksum(
    path: &Path,
    checksum_type: &ChecksumType,
    expected_sha256: Option<&str>,
) -> Result<(), String> {
    match checksum_type {
        ChecksumType::None => Ok(()),
        ChecksumType::Sha256 => {
            let expected = expected_sha256.ok_or("SHA256 checksum expected but not provided")?;
            let data =
                fs::read(path).map_err(|e| format!("Cannot read file for checksum: {}", e))?;
            let hash = sha256_digest(&data);
            if hash != expected.trim().to_lowercase() {
                // Clean up bad file
                let _ = fs::remove_file(path);
                Err(format!(
                    "Checksum mismatch.\n  Expected: {}\n  Got:      {}",
                    expected.trim(),
                    hash
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn sha256_digest(data: &[u8]) -> String {
    use sha2::{digest::Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Extract an archive to destination directory.
pub fn extract_archive(
    archive: &Path,
    dest: &Path,
    format: &crate::domain::ArchiveFormat,
) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("Cannot create dest dir: {}", e))?;

    match format {
        crate::domain::ArchiveFormat::TarGz
        | crate::domain::ArchiveFormat::TarXz
        | crate::domain::ArchiveFormat::TarBz2 => extract_tar(archive, dest, format),
        crate::domain::ArchiveFormat::Zip => extract_zip(archive, dest),
    }
}

fn extract_tar(
    archive: &Path,
    dest: &Path,
    format: &crate::domain::ArchiveFormat,
) -> Result<(), String> {
    let f = fs::File::open(archive).map_err(|e| format!("Cannot open archive: {}", e))?;
    let decoder: Box<dyn io::Read> = match format {
        crate::domain::ArchiveFormat::TarXz => Box::new(xz2::read::XzDecoder::new(f)),
        crate::domain::ArchiveFormat::TarBz2 => Box::new(bzip2::read::BzDecoder::new(f)),
        _ => Box::new(flate2::read::GzDecoder::new(f)),
    };
    let mut archive_reader = tar::Archive::new(decoder);

    // tar archives often have a top-level directory; strip it
    for entry in archive_reader
        .entries()
        .map_err(|e| format!("tar error: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("tar entry error: {}", e))?;
        let entry_path = entry.path().map_err(|e| format!("path error: {}", e))?;

        // Strip first component (e.g. "jdk-21.0.1.jdk/" → "")
        let components: Vec<_> = entry_path.components().collect();
        if components.len() <= 1 {
            continue;
        }
        let stripped: PathBuf = components[1..].iter().collect();
        let target = dest.join(stripped);

        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
            // Remove existing file/symlink if present
            let _ = fs::remove_file(&target);
            if let Ok(Some(link_target)) = entry.link_name() {
                let _ = std::os::unix::fs::symlink(&link_target, &target);
            }
        } else if entry_type.is_dir() {
            let _ = fs::create_dir_all(&target);
        } else {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut f = fs::File::create(&target).map_err(|e| format!("create: {}", e))?;
            io::copy(&mut entry, &mut f).map_err(|e| format!("copy: {}", e))?;
        }
    }
    Ok(())
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let f = fs::File::open(archive).map_err(|e| format!("Cannot open archive: {}", e))?;
    let mut zip_archive = zip::ZipArchive::new(f).map_err(|e| format!("zip error: {}", e))?;

    for i in 0..zip_archive.len() {
        let mut entry = zip_archive
            .by_index(i)
            .map_err(|e| format!("zip entry: {}", e))?;
        let entry_path = entry.mangled_name();

        // Strip first component
        let components: Vec<_> = entry_path.components().collect();
        if components.len() <= 1 && entry.is_dir() {
            continue;
        }
        let stripped: PathBuf = if components.len() > 1 {
            components[1..].iter().collect()
        } else {
            entry_path.clone()
        };

        let target = dest.join(stripped);
        if entry.is_dir() {
            let _ = fs::create_dir_all(&target);
        } else {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut f = fs::File::create(&target).map_err(|e| format!("create: {}", e))?;
            io::copy(&mut entry, &mut f).map_err(|e| format!("copy: {}", e))?;
        }
    }
    Ok(())
}
