use std::path::Path;
use std::process::Command;
use std::fs;
use crate::parser::{Rule, Task};
use crate::run::cache;
use crate::file;
use crate::parser::reader::config as reader;

fn substitute_variables(command: &str, rule: &Rule) -> anyhow::Result<String> {
    let mut result = command.to_string();

    let outputs: Vec<&str> = rule.actions.iter().map(|a| a.output.as_str()).collect();
    result = result.replace("$(output)", &outputs.join(" "));

    let mut input_files: Vec<String> = Vec::new();
    for action in &rule.actions {
        let path = Path::new(&action.input);
        if path.is_dir() {
            collect_files(path, &mut input_files)?;
        } else if path.is_file() {
            input_files.push(action.input.clone());
        }
    }
    input_files.sort();
    result = result.replace("$(input)", &input_files.join(" "));

    Ok(result)
}

fn collect_files(dir: &Path, files: &mut Vec<String>) -> anyhow::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(s) = path.to_str() {
                    files.push(s.to_string());
                }
            } else if path.is_dir() {
                collect_files(&path, files)?;
            }
        }
    }
    Ok(())
}

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
            println!("[bee/info] Skipping task: {} (up-to-date)", task.name);
            return Ok(());
        }

        println!("[bee/info] Running task: {}", task.name);

        let command_str = substitute_variables(&task.run, &rule)?;
        let command = run_command(&command_str)?;
        print!("[bee/info] {}", command.0);
        if !command.0.ends_with('\n') {
            println!();
        }
        eprint!("[bee/error] {}", command.1);
        if !command.1.ends_with('\n') {
            eprintln!();
        }

        cache::save_run_hash(&task.name, &current_hash)?;
    } else {
        println!("[bee/info] Running task: {}", task.name);
        
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