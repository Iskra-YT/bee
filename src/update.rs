use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const REPO: &str = "Iskra-YT/bee";
const API_BASE: &str = "https://api.github.com";

pub fn run_update() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let platform = detect_platform()?;
    println!("  Detected platform: {}", platform.artifact_suffix);

    let release = fetch_latest_bee_release()?;
    let tag = release["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow!("Latest release has no tag_name"))?;
    let sha = parse_release_sha(tag)?;
    println!("  Latest release: {} (commit {})", tag, sha);

    let asset_name = format!("bee-{}-{}.{}", sha, platform.artifact_suffix, platform.ext);
    let asset_url = find_asset_url(&release, &asset_name)?;
    let checksums_url = find_asset_url(&release, "SHA256SUMS").ok();

    let tmp_dir = make_tmp_dir()?;
    let archive_path = tmp_dir.join(&asset_name);
    let checksums_path = tmp_dir.join("SHA256SUMS");

    println!("  Downloading {}...", asset_name);
    download_file(&asset_url, &archive_path)?;

    if let Some(url) = checksums_url {
        println!("  Downloading SHA256SUMS...");
        download_file(&url, &checksums_path)?;
        verify_checksum(&archive_path, &checksums_path, &asset_name)?;
        println!("  Checksum verified.");
    } else {
        println!("  Warning: SHA256SUMS asset not found; skipping checksum verification.");
    }

    let extract_dir = tmp_dir.join("extract");
    fs::create_dir_all(&extract_dir)?;
    extract_archive(&archive_path, &extract_dir, platform.ext)?;

    let binary_name = if platform.is_windows { "bee.exe" } else { "bee" };
    let extracted_binary = find_extracted_binary(&extract_dir, binary_name)?;
    install_binary(&extracted_binary, &current_exe)?;

    let _ = fs::remove_dir_all(&tmp_dir);

    println!(
        "  bee updated successfully to {} (release {}, commit {})",
        current_exe.display(),
        tag,
        sha
    );
    println!("  run 'bee version' to confirm");
    Ok(())
}

struct Platform {
    artifact_suffix: &'static str,
    ext: &'static str,
    is_windows: bool,
}

fn detect_platform() -> Result<Platform> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let (artifact_suffix, ext, is_windows) = match (os, arch) {
        ("linux", "x86_64") => ("linux-x64", "tar.gz", false),
        ("linux", "aarch64") => ("linux-arm64", "tar.gz", false),
        ("macos", "x86_64") => ("macos-x64", "tar.gz", false),
        ("macos", "aarch64") => ("macos-arm64", "tar.gz", false),
        ("windows", "x86_64") => ("windows-x64", "zip", true),
        ("freebsd", "x86_64") => ("freebsd-x64", "tar.gz", false),
        _ => {
            return Err(anyhow!(
                "Unsupported platform: {}-{}. Supported: linux-x64, linux-arm64, macos-x64, macos-arm64, windows-x64, freebsd-x64",
                os, arch
            ))
        }
    };
    Ok(Platform {
        artifact_suffix,
        ext,
        is_windows,
    })
}

fn fetch_latest_bee_release() -> Result<json::JsonValue> {
    let mut page = 1u32;
    loop {
        let url = format!("{}/repos/{}/releases?per_page=100&page={}", API_BASE, REPO, page);
        let body = curl_get(&url)?;
        let parsed = json::parse(&body)
            .map_err(|e| anyhow!("Failed to parse GitHub releases response: {}", e))?;
        let arr = match parsed {
            json::JsonValue::Array(a) => a,
            _ => return Err(anyhow!("Unexpected GitHub releases response format")),
        };
        if arr.is_empty() {
            break;
        }
        for release in &arr {
            if release["draft"].as_bool().unwrap_or(false)
                || release["prerelease"].as_bool().unwrap_or(false)
            {
                continue;
            }
            if let Some(tag) = release["tag_name"].as_str() {
                if parse_release_sha(tag).is_ok() {
                    return Ok(release.clone());
                }
            }
        }
        if arr.len() < 100 {
            break;
        }
        page += 1;
        if page > 10 {
            return Err(anyhow!("Too many release pages; refusing to continue"));
        }
    }
    Err(anyhow!(
        "No published Bee release found (expected tag bee-<40-char-sha>) in {}/{}",
        "https://github.com",
        REPO
    ))
}

fn parse_release_sha(tag: &str) -> Result<String> {
    let prefix = "bee-";
    let sha = tag
        .strip_prefix(prefix)
        .ok_or_else(|| anyhow!("Release tag '{}' does not start with 'bee-'", tag))?;
    if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(sha.to_string())
    } else {
        Err(anyhow!(
            "Release tag '{}' does not contain a full 40-character commit SHA",
            tag
        ))
    }
}

fn find_asset_url(release: &json::JsonValue, name: &str) -> Result<String> {
    if let json::JsonValue::Array(assets) = &release["assets"] {
        for asset in assets {
            if asset["name"].as_str() == Some(name) {
                if let Some(url) = asset["browser_download_url"].as_str() {
                    return Ok(url.to_string());
                }
            }
        }
    }
    Err(anyhow!("Release asset '{}' not found", name))
}

fn curl_get(url: &str) -> Result<String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "-H", "Accept: application/vnd.github+json", "-H", "User-Agent: bee-updater"]);
    if let Ok(token) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")) {
        if !token.trim().is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", token)]);
        }
    }
    cmd.arg(url);
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow!("curl not found; bee update requires curl to download releases")
        } else {
            anyhow!("Failed to run curl: {}", e)
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "curl GET failed (exit code {}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fL", "--progress-bar", "-o"]);
    cmd.arg(dest);
    cmd.args(["-H", "User-Agent: bee-updater"]);
    if let Ok(token) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")) {
        if !token.trim().is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", token)]);
        }
    }
    cmd.arg(url);
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let status = cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow!("curl not found; bee update requires curl to download releases")
        } else {
            anyhow!("Failed to run curl: {}", e)
        }
    })?;
    if !status.success() {
        let _ = fs::remove_file(dest);
        return Err(anyhow!(
            "Download failed (exit code {}) for {}",
            status.code().unwrap_or(-1),
            url
        ));
    }
    Ok(())
}

fn verify_checksum(archive: &Path, checksums_file: &Path, asset_name: &str) -> Result<()> {
    let checksums = fs::read_to_string(checksums_file)?;
    let expected = checksums
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?;
            let name = name.trim_start_matches('*');
            if name == asset_name {
                Some(hash.to_string())
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| anyhow!("No checksum found for {} in SHA256SUMS", asset_name))?;

    let mut file = fs::File::open(archive)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual.eq_ignore_ascii_case(&expected) {
        Ok(())
    } else {
        Err(anyhow!(
            "Checksum mismatch for {}: expected {}, got {}",
            asset_name,
            expected,
            actual
        ))
    }
}

fn extract_archive(archive: &Path, dest: &Path, ext: &str) -> Result<()> {
    let status = if ext == "zip" {
        Command::new("unzip")
            .args(["-o"])
            .arg(archive)
            .arg("-d")
            .arg(dest)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    } else {
        Command::new("tar")
            .args(["-xzf"])
            .arg(archive)
            .arg("-C")
            .arg(dest)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    };
    let status = status.map_err(|e| anyhow!("Failed to extract archive: {}", e))?;
    if !status.success() {
        return Err(anyhow!(
            "Archive extraction failed (exit code {})",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

fn find_extracted_binary(extract_dir: &Path, binary_name: &str) -> Result<PathBuf> {
    // Common layout: <artifact>/bee
    let direct = extract_dir.join(binary_name);
    if direct.is_file() {
        return Ok(direct);
    }
    for entry in fs::read_dir(extract_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join(binary_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(anyhow!(
        "Extracted binary '{}' not found in {}",
        binary_name,
        extract_dir.display()
    ))
}

fn install_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    println!("  Installing to {}...", current_exe.display());
    let backup = current_exe.with_extension("old");

    // Replace existing binary. On Windows, renaming running exe is allowed; removing may not be.
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    if current_exe.exists() {
        fs::rename(current_exe, &backup).or_else(|_| {
            fs::remove_file(current_exe)?;
            Ok::<(), std::io::Error>(())
        })?;
    }

    fs::copy(new_binary, current_exe)?;
    set_executable(current_exe)?;

    // Best effort cleanup of backup
    let _ = fs::remove_file(&backup);
    Ok(())
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn make_tmp_dir() -> Result<PathBuf> {
    let base = std::env::temp_dir().join(format!(
        "bee-update-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::create_dir_all(&base)?;
    Ok(base)
}
