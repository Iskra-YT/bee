use std::fs;
use std::process::Command;
use anyhow::Result;

pub fn install(name: &str, source: &str, version: Option<&str>) -> Result<()> {
    let deps_dir = format!("bee/deps/{}", name);

    if std::path::Path::new(&deps_dir).exists() {
        println!("  {} already exists, pulling latest...", name);
        let pull_output = Command::new("git")
            .args(["-C", &deps_dir, "pull"])
            .output()?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            return Err(anyhow::anyhow!("Failed to pull {}: {}", name, stderr));
        }
    } else {
        fs::create_dir_all("bee/deps")?;

        let mut args = vec!["clone", "--depth", "1", source, &deps_dir];

        let version_str;
        if let Some(v) = version {
            version_str = format!("refs/tags/{}", v);
            args = vec!["clone", "--depth", "1", "--branch", &version_str, source, &deps_dir];
        }

        println!("  Cloning {} from {}...", name, source);
        let output = Command::new("git")
            .args(&args)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to clone {}: {}", name, stderr));
        }
    }

    println!("  Installed {} to {}", name, deps_dir);
    Ok(())
}
