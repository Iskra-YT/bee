use anyhow::Result;
use std::process::Command;

pub fn install(name: &str, source: &str, command: Option<&str>) -> Result<()> {
    let install_cmd = command.unwrap_or("npm install -g");

    println!("  Installing {} via '{}'...", name, install_cmd);

    let parts: Vec<&str> = install_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty install command for {}", name));
    }

    let mut args: Vec<&str> = parts[1..].to_vec();
    args.push(source);

    let output = Command::new(parts[0]).args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to install {}: {}", name, stderr));
    }

    println!("  Installed {}", name);
    Ok(())
}
