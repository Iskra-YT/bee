use std::process::Command;
use crate::parser::Task;

pub fn run_task(task: Task) -> std::io::Result<(String, String)> {
    println!("Running task: {}", task.name);
    run_command(&task.run)
}

pub fn run_command(command: &str) -> std::io::Result<(String, String)> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;

    Ok((String::from_utf8_lossy(&output.stdout).to_string(), String::from_utf8_lossy(&output.stderr).to_string()))
}