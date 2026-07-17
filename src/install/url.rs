use std::fs;
use std::process::Command;
use anyhow::Result;

pub fn install(name: &str, source: &str, _version: Option<&str>) -> Result<()> {
    let deps_dir = format!("bee/deps/{}", name);
    fs::create_dir_all(&deps_dir)?;

    println!("  Downloading {} from {}...", name, source);

    let output = Command::new("curl")
        .args(["-L", "-o", &format!("{}/download", deps_dir), source])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to download {}: {}", name, stderr));
    }

    if source.ends_with(".tar.gz") || source.ends_with(".tgz") {
        println!("  Extracting {}...", name);
        let extract_output = Command::new("tar")
            .args(["-xzf", &format!("{}/download", deps_dir), "-C", &deps_dir])
            .output()?;

        if !extract_output.status.success() {
            let stderr = String::from_utf8_lossy(&extract_output.stderr);
            return Err(anyhow::anyhow!("Failed to extract {}: {}", name, stderr));
        }
        let _ = fs::remove_file(format!("{}/download", deps_dir));
    } else if source.ends_with(".zip") {
        println!("  Extracting {}...", name);
        let extract_output = Command::new("unzip")
            .args(["-o", &format!("{}/download", deps_dir), "-d", &deps_dir])
            .output()?;

        if !extract_output.status.success() {
            let stderr = String::from_utf8_lossy(&extract_output.stderr);
            return Err(anyhow::anyhow!("Failed to extract {}: {}", name, stderr));
        }
        let _ = fs::remove_file(format!("{}/download", deps_dir));
    }

    println!("  Installed {} to {}", name, deps_dir);
    Ok(())
}
