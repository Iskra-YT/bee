use std::process::Command;
use anyhow::Result;

pub fn install(name: &str, command: &str) -> Result<()> {
    println!("  Running '{}' for {}...", command, name);

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        print!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        return Err(anyhow::anyhow!("Command '{}' failed with exit code {} for {}", command, code, name));
    }

    println!("  Completed '{}'", name);
    Ok(())
}
