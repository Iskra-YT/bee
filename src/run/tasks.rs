use std::process::Command;
use crate::parser::Task;
use crate::run::cache;
use crate::file;
use crate::parser::reader::config as reader;

pub fn run_task(task: Task) -> anyhow::Result<()> {
    let rule_file = format!("bee/rules/{}.yml", task.name);
    let rule = file::get_file_content(&rule_file)
        .ok()
        .and_then(|content| reader::read_rules_from_string(&content).ok());

    if let Some(rule) = rule {
        let current_hash = cache::calculate_input_hash(&rule)?;
        let last_hash = cache::get_last_run_hash(&task.name);
        let outputs_ok = cache::check_outputs_exist(&rule);

        if Some(current_hash.clone()) == last_hash && outputs_ok {
            println!("Skipping task: {} (up-to-date)", task.name);
            return Ok(());
        }

        println!("Running task: {}", task.name);

        let command = run_command(&task.run)?;
        print!("{}", command.0);
        eprint!("{}", command.1);

        cache::save_run_hash(&task.name, &current_hash)?;
    } else {
        println!("Running task: {}", task.name);
        
        let command = run_command(&task.run)?;
        print!("{}", command.0);
        eprint!("{}", command.1);
    }

    Ok(())
}

pub fn run_command(command: &str) -> anyhow::Result<(String, String)> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Command failed: {}", stderr));
    }

    Ok((String::from_utf8_lossy(&output.stdout).to_string(), String::from_utf8_lossy(&output.stderr).to_string()))
}