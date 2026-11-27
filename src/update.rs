//! Fail-closed self-update from GitHub Releases.
//!
//! An update is only installed when a `SHA256SUMS` asset is present and the
//! downloaded archive matches it. The new binary is smoke-tested both before
//! and after the swap; a failed smoke test restores the previous executable.

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

    // Fail closed: without a signed-off checksum list there is no update.
    let checksums_url =
        find_asset_url(&release, "SHA256SUMS").map_err(|e| anyhow!("refusing to update: {}", e))?;
    let tmp_dir = make_tmp_dir()?;
    let archive_path = tmp_dir.join(&asset_name);
    let checksums_path = tmp_dir.join("SHA256SUMS");

    println!("  Downloading SHA256SUMS...");
    download_file(&checksums_url, &checksums_path)?;
    let expected = checksum_for(&fs::read_to_string(&checksums_path)?, &asset_name)?;
    println!("  Expected SHA-256: {}", expected);

    println!("  Downloading {}...", asset_name);
    download_file(&asset_url, &archive_path)?;
    verify_checksum(&archive_path, &checksums_path, &asset_name)?;
    println!("  Checksum verified.");

    let extract_dir = tmp_dir.join("extract");
    fs::create_dir_all(&extract_dir)?;
    extract_archive(&archive_path, &extract_dir, platform.ext)?;

    let binary_name = if platform.is_windows {
        "bee.exe"
    } else {
        "bee"
    };
    let extracted_binary = find_extracted_binary(&extract_dir, binary_name)?;
    println!("  Smoke-testing the downloaded binary...");
    smoketest(&extracted_binary, Some(&sha))?;

    let installed = install_binary(&extracted_binary, &current_exe, Some(&sha))?;
    let _ = fs::remove_dir_all(&tmp_dir);

    if installed {
        println!(
            "  bee updated successfully to {} (release {}, commit {})",
            current_exe.display(),
            tag,
            sha
        );
        println!("  run 'bee version' to confirm");
    } else {
        println!(
            "  The new binary is staged at '{}' because Windows keeps the running executable locked.",
            staged_path(&current_exe).display()
        );
        println!(
            "  Run '{}' once to finish the update.",
            helper_path(&current_exe).display()
        );
    }
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

/// Wybiera najnowsze opublikowane wydanie, niezależnie od kolejności zwróconej przez API.
fn pick_latest_release(releases: &[json::JsonValue]) -> Option<&json::JsonValue> {
    releases
        .iter()
        .filter(|release| {
            !release["draft"].as_bool().unwrap_or(false)
                && !release["prerelease"].as_bool().unwrap_or(false)
                && release["tag_name"]
                    .as_str()
                    .is_some_and(|tag| parse_release_sha(tag).is_ok())
        })
        .max_by(|left, right| {
            left["published_at"]
                .as_str()
                .unwrap_or("")
                .cmp(right["published_at"].as_str().unwrap_or(""))
        })
}

fn fetch_latest_bee_release() -> Result<json::JsonValue> {
    let mut page = 1u32;
    let mut releases = Vec::new();
    loop {
        let url = format!(
            "{}/repos/{}/releases?per_page=100&page={}",
            API_BASE, REPO, page
        );
        let body = curl_get(&url)?;
        let parsed = json::parse(&body)
            .map_err(|e| anyhow!("Failed to parse GitHub releases response: {}", e))?;
        let arr = match parsed {
            json::JsonValue::Array(a) => a,
            _ => return Err(anyhow!("Unexpected GitHub releases response format")),
        };
        let exhausted = arr.len() < 100;
        releases.extend(arr);
        if exhausted {
            break;
        }
        page += 1;
        if page > 10 {
            return Err(anyhow!("Too many release pages; refusing to continue"));
        }
    }
    pick_latest_release(&releases).cloned().ok_or_else(|| {
        anyhow!(
            "No published Bee release found (expected tag bee-<40-char-sha>) in https://github.com/{}",
            REPO
        )
    })
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
    cmd.args([
        "-fsSL",
        "-H",
        "Accept: application/vnd.github+json",
        "-H",
        "User-Agent: bee-updater",
    ]);
    add_auth_header(&mut cmd);
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

fn add_auth_header(cmd: &mut Command) {
    if let Ok(token) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")) {
        if !token.trim().is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", token)]);
        }
    }
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fL", "--progress-bar", "-o"]);
    cmd.arg(dest);
    cmd.args(["-H", "User-Agent: bee-updater"]);
    add_auth_header(&mut cmd);
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

/// Look up the expected digest in SHA256SUMS. Missing entries are an error.
pub fn checksum_for(checksums: &str, asset_name: &str) -> Result<String> {
    let expected = checksums
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            if name == asset_name {
                Some(hash.to_string())
            } else {
                None
            }
        })
        .find(|hash| hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| {
            anyhow!(
                "SHA256SUMS has no valid 64-character SHA-256 entry for '{}'",
                asset_name
            )
        })?;
    Ok(expected)
}

pub fn verify_checksum(archive: &Path, checksums_file: &Path, asset_name: &str) -> Result<()> {
    let checksums = fs::read_to_string(checksums_file)?;
    let expected = checksum_for(&checksums, asset_name)?;

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
    let mut cmd = if ext == "zip" {
        let mut cmd = Command::new("unzip");
        cmd.arg("-o").arg(archive).arg("-d").arg(dest);
        cmd
    } else {
        let mut cmd = Command::new("tar");
        cmd.arg("-xzf").arg(archive).arg("-C").arg(dest);
        cmd
    };
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let status = cmd
        .status()
        .map_err(|e| anyhow!("Failed to extract archive: {}", e))?;
    if !status.success() {
        return Err(anyhow!(
            "Archive extraction failed (exit code {})",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

fn find_extracted_binary(extract_dir: &Path, binary_name: &str) -> Result<PathBuf> {
    let candidates = vec![extract_dir.join(binary_name)].into_iter().chain(
        fs::read_dir(extract_dir)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|entry| entry.path().join(binary_name)),
    );
    let canonical_dir = fs::canonicalize(extract_dir)?;
    for candidate in candidates {
        if !candidate.is_file() {
            continue;
        }
        let canonical = fs::canonicalize(&candidate)?;
        if !canonical.starts_with(&canonical_dir) {
            return Err(anyhow!(
                "extracted binary '{}' escapes the extraction directory",
                candidate.display()
            ));
        }
        return Ok(canonical);
    }
    Err(anyhow!(
        "Extracted binary '{}' not found in {}",
        binary_name,
        extract_dir.display()
    ))
}

/// Run `bee version` on a candidate binary. A release binary must report the
/// commit SHA it was built from; unknown or missing output is a hard failure.
pub fn smoketest(binary: &Path, expected_sha: Option<&str>) -> Result<()> {
    let output = Command::new(binary)
        .arg("version")
        .output()
        .map_err(|e| anyhow!("smoke test could not start '{}': {}", binary.display(), e))?;
    if !output.status.success() {
        return Err(anyhow!(
            "smoke test failed: '{} version' exited with code {}",
            binary.display(),
            output.status.code().unwrap_or(-1)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let reported = stdout.trim();
    if reported.is_empty() {
        return Err(anyhow!(
            "smoke test failed: '{} version' printed nothing",
            binary.display()
        ));
    }
    if let Some(expected) = expected_sha {
        let found = reported
            .split_whitespace()
            .find(|token| token.len() == 40 && token.chars().all(|c| c.is_ascii_hexdigit()));
        match found {
            Some(found) if found.eq_ignore_ascii_case(expected) => Ok(()),
            Some(found) => Err(anyhow!(
                "smoke test failed: binary reports commit {}, expected {}",
                found,
                expected
            )),
            None => Err(anyhow!(
                "smoke test failed: binary reports '{}', expected commit {}",
                reported,
                expected
            )),
        }
    } else {
        Ok(())
    }
}

fn staged_path(current_exe: &Path) -> PathBuf {
    let name = current_exe
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "bee".to_string());
    let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&name);
    current_exe.with_file_name(format!("{}-new.exe", stem))
}

fn helper_path(current_exe: &Path) -> PathBuf {
    current_exe.with_file_name("bee.cmd")
}

/// Write the Windows helper that completes a staged update after the running
/// executable is closed.
pub fn write_windows_helper(current_exe: &Path) -> Result<PathBuf> {
    let exe_name = current_exe
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("cannot determine executable name"))?;
    let staged_name = staged_path(current_exe)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("cannot determine staged binary name"))?;
    let helper = helper_path(current_exe);
    let script = format!(
        "@echo off\nsetlocal\nset \"BEE_DIR=%~dp0\"\n\
         if not exist \"%BEE_DIR%{staged}\" (\n  echo bee: no staged update found >&2\n  exit /b 1\n)\n\
         move /Y \"%BEE_DIR%{staged}\" \"%BEE_DIR%{exe}\" >nul\n\
         if errorlevel 1 (\n  echo bee: could not replace {exe}; close bee and run this helper again >&2\n  exit /b 1\n)\n\
         \"%BEE_DIR%{exe}\" %*\n",
        staged = staged_name,
        exe = exe_name
    );
    fs::write(&helper, script)?;
    Ok(helper)
}

fn copy_for_execution(src: &Path, dst: &Path) -> Result<()> {
    let mut input = fs::File::open(src)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    drop(output);
    drop(input);
    Ok(())
}

/// Install a verified binary. Returns `false` when the update was staged for
/// the Windows helper because the running executable could not be replaced.
pub fn install_binary(
    new_binary: &Path,
    current_exe: &Path,
    expected_sha: Option<&str>,
) -> Result<bool> {
    let parent = current_exe
        .parent()
        .ok_or_else(|| anyhow!("cannot determine the directory of the current executable"))?;
    let stamp = chrono::Utc::now().timestamp_millis();
    let temp = parent.join(format!(".bee-update-{}-{}", std::process::id(), stamp));
    let backup = parent.join(format!(
        "{}.old-{}",
        current_exe
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "bee".to_string()),
        stamp
    ));

    copy_for_execution(new_binary, &temp)?;
    set_executable(&temp)?;
    smoketest(&temp, expected_sha)?;
    println!("  Installing to {}...", current_exe.display());

    if let Err(rename_err) = fs::rename(current_exe, &backup) {
        #[cfg(windows)]
        {
            // The running exe is locked on Windows. Stage it and let bee.cmd
            // finish the swap on the next invocation.
            fs::copy(&temp, staged_path(current_exe))?;
            let helper = write_windows_helper(current_exe)?;
            fs::remove_file(&temp)?;
            println!("  Could not replace the running executable: {}", rename_err);
            println!("  Run '{}' to finish the update.", helper.display());
            return Ok(false);
        }
        #[cfg(not(windows))]
        {
            fs::remove_file(&temp)?;
            return Err(anyhow!(
                "could not back up the current executable: {}; bee was not modified",
                rename_err
            ));
        }
    }

    if let Err(swap_err) = fs::rename(&temp, current_exe) {
        // Restore the original binary before reporting the failure.
        let rollback = fs::rename(&backup, current_exe);
        let _ = fs::remove_file(&temp);
        return Err(anyhow!(
            "could not install the new binary: {}; rollback: {:?}",
            swap_err,
            rollback
        ));
    }

    if let Err(err) = smoketest(current_exe, expected_sha) {
        // Post-swap failure: restore the previous binary.
        let _ = fs::remove_file(current_exe);
        if let Err(rollback_err) = fs::rename(&backup, current_exe) {
            return Err(anyhow!(
                "new binary failed its smoke test ({}) AND rollback failed ({}); restore the backup at {} manually",
                err,
                rollback_err,
                backup.display()
            ));
        }
        set_executable(current_exe)?;
        return Err(anyhow!(
            "new binary failed its smoke test ({}); the previous executable was restored",
            err
        ));
    }

    // Best effort: the update already succeeded and was smoke-tested.
    let _ = fs::remove_file(&backup);
    Ok(true)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_release_is_chosen_by_publication_date_not_response_order() {
        let old_sha = "a".repeat(40);
        let new_sha = "b".repeat(40);
        let draft_sha = "c".repeat(40);
        let payload = format!(
            r#"[
                {{"tag_name":"bee-{old_sha}","draft":false,"prerelease":false,"published_at":"2026-11-27T09:03:01Z"}},
                {{"tag_name":"bee-{draft_sha}","draft":true,"prerelease":false,"published_at":"2026-11-27T10:30:00Z"}},
                {{"tag_name":"bee-{new_sha}","draft":false,"prerelease":false,"published_at":"2026-11-27T10:01:45Z"}}
            ]"#
        );
        let parsed = json::parse(&payload).unwrap();
        let releases = match parsed {
            json::JsonValue::Array(releases) => releases,
            _ => panic!("expected an array"),
        };
        let latest = pick_latest_release(&releases).unwrap();
        assert_eq!(
            latest["tag_name"].as_str(),
            Some(format!("bee-{new_sha}").as_str())
        );
    }

    #[test]
    fn releases_without_bee_tag_are_ignored() {
        let payload = r#"[
            {"tag_name":"v1.0.0","draft":false,"prerelease":false,"published_at":"2026-11-28T00:00:00Z"},
            {"tag_name":"prerelease","draft":false,"prerelease":true,"published_at":"2026-11-29T00:00:00Z"}
        ]"#;
        let parsed = json::parse(payload).unwrap();
        let releases = match parsed {
            json::JsonValue::Array(releases) => releases,
            _ => panic!("expected an array"),
        };
        assert!(pick_latest_release(&releases).is_none());
    }

    #[test]
    fn checksum_lookup_requires_a_valid_entry() {
        let sums = "abc  other.tar.gz\ndeadbeef  bee.tar.gz\n";
        assert!(checksum_for(sums, "bee.tar.gz").is_err());
        let hash = "a".repeat(64);
        let sums = format!("{}  bee.tar.gz\n", hash);
        assert_eq!(checksum_for(&sums, "bee.tar.gz").unwrap(), hash);
        assert!(checksum_for(&sums, "missing.tar.gz").is_err());
    }

    #[test]
    fn windows_helper_mentions_both_binaries() {
        let exe = Path::new("/tmp/tools/bee.exe");
        assert_eq!(
            staged_path(exe).file_name().and_then(|n| n.to_str()),
            Some("bee-new.exe")
        );
        assert_eq!(
            helper_path(exe).file_name().and_then(|n| n.to_str()),
            Some("bee.cmd")
        );
        let dir = tempfile::tempdir().unwrap();
        let helper = write_windows_helper(&dir.path().join("bee.exe")).unwrap();
        let content = fs::read_to_string(helper).unwrap();
        assert!(content.contains("bee-new.exe"));
        assert!(content.contains("bee.exe"));
    }
}
