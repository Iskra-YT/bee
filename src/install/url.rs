use anyhow::Result;
use std::fs;
use std::process::{Command, Stdio};
use url::Url;

fn validate_archive_members(program: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(program).args(args).output();
    let Ok(output) = output else {
        return Ok(()); // extraction command itself will report the failure
    };
    if !output.status.success() {
        return Ok(());
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let path = std::path::Path::new(line.trim());
        if path.is_absolute()
            || path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            })
        {
            return Err(anyhow::anyhow!(
                "archive contains an unsafe path ('{}'); refusing to extract",
                line.trim()
            ));
        }
    }
    Ok(())
}

fn filename_from_url(source: &str) -> Result<String> {
    if let Ok(url) = Url::parse(source) {
        if let Some(mut segments) = url.path_segments() {
            if let Some(last) = segments.next_back() {
                if !last.is_empty() {
                    return Ok(last.to_string());
                }
            }
        }
    }
    Ok("download".to_string())
}

pub fn install(name: &str, source: &str, version: Option<&str>, update: bool) -> Result<bool> {
    crate::safety::validate_name("dependency", name)?;
    let deps_dir = crate::safety::safe_path("bee/deps", "dependency", name)?;
    let deps_dir_str = deps_dir.to_string_lossy().to_string();
    let version_marker = deps_dir.join(".bee-version");

    if deps_dir.exists() && !update {
        let has_files = fs::read_dir(&deps_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.file_name() != std::ffi::OsStr::new(".bee-version"))
            })
            .unwrap_or(false);
        let version_matches = match version {
            Some(expected) => fs::read_to_string(&version_marker)
                .map(|actual| actual.trim() == expected)
                .unwrap_or(false),
            None => true,
        };
        if has_files && version_matches {
            println!("  {} already installed, skipping.", name);
            return Ok(false);
        }
        if has_files && !version_matches {
            println!(
                "  {} is installed at a different version, reinstalling.",
                name
            );
        }
    }

    fs::create_dir_all(&deps_dir)?;

    let filename = filename_from_url(source)?;
    let download_path = deps_dir.join(filename);
    let download_path_str = download_path.to_string_lossy().to_string();

    println!("  Downloading {} from {}...", name, source);
    let status = Command::new("curl")
        .args([
            "-fL",
            "--retry",
            "3",
            "--connect-timeout",
            "15",
            "--max-time",
            "300",
            "--progress-bar",
            "-o",
            &download_path_str,
            source,
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        let _ = fs::remove_file(&download_path);
        return Err(anyhow::anyhow!(
            "Failed to download {} (exit code {})",
            name,
            code
        ));
    }
    eprintln!();

    if source.ends_with(".tar.gz") || source.ends_with(".tgz") {
        validate_archive_members("tar", &["-tzf", &download_path_str])?;
        println!("  Extracting {}...", name);
        let extract_status = Command::new("tar")
            .args(["-xzf", &download_path_str, "-C", &deps_dir_str])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !extract_status.success() {
            let code = extract_status.code().unwrap_or(-1);
            return Err(anyhow::anyhow!(
                "Failed to extract {} (exit code {})",
                name,
                code
            ));
        }
        let _ = fs::remove_file(&download_path);
    } else if source.ends_with(".zip") {
        validate_archive_members("unzip", &["-Z1", &download_path_str])?;
        println!("  Extracting {}...", name);
        let extract_status = Command::new("unzip")
            .args(["-o", &download_path_str, "-d", &deps_dir_str])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !extract_status.success() {
            let code = extract_status.code().unwrap_or(-1);
            return Err(anyhow::anyhow!(
                "Failed to extract {} (exit code {})",
                name,
                code
            ));
        }
        let _ = fs::remove_file(&download_path);
    }

    match version {
        Some(expected) => fs::write(&version_marker, expected)?,
        None => {
            if version_marker.exists() {
                fs::remove_file(&version_marker)?;
            }
        }
    }

    println!("  Installed {} to {}", name, deps_dir.display());
    Ok(true)
}
