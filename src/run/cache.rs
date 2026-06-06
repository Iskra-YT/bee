use crate::parser::Rule;
use crate::hash;
use std::fs;
use std::path::Path;
use anyhow::Result;

pub fn calculate_input_hash(rule: &Rule) -> Result<String> {
    let mut combined_hash = String::new();

    for action in &rule.actions {
        let path = Path::new(&action.input);
        if path.is_file() {
            if let Ok(h) = hash::hash_file(action.input.as_str()) {
                combined_hash.push_str(&h);
                let _ = save_file_hash(&rule.task, &action.input, &h);
            }
        } else if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                entries.sort_by_key(|e| e.path());

                for entry in entries {
                    if entry.path().is_file() {
                        let file_path = entry.path().to_str().unwrap_or_default().to_string();
                        if let Ok(h) = hash::hash_file(&file_path) {
                            combined_hash.push_str(&h);
                            let _ = save_file_hash(&rule.task, &file_path, &h);
                        }
                    }
                }
            }
        }
    }

    if combined_hash.is_empty() {
        return Ok(String::from("no-inputs"));
    }

    Ok(hash::hash_string(&combined_hash))
}

fn save_file_hash(task_name: &str, file_path: &str, hash: &str) -> Result<()> {
    let safe_file_name = file_path.replace("/", "_").replace("\\", "_");
    let dir = format!("bee/cache/{}/inputs", task_name);
    fs::create_dir_all(&dir)?;

    let full_path = format!("{}/{}", dir, safe_file_name);
    fs::write(full_path, hash)?;
    Ok(())
}

pub fn check_outputs_exist(rule: &Rule) -> bool {
    for action in &rule.actions {
        if !Path::new(&action.output).exists() {
            return false;
        }
    }
    true
}

pub fn get_last_run_hash(task_name: &str) -> Option<String> {
    let path = format!("bee/cache/{}/last_run", task_name);
    fs::read_to_string(path).ok()
}

pub fn save_run_hash(task_name: &str, hash: &str) -> Result<()> {
    let dir = format!("bee/cache/{}", task_name);
    fs::create_dir_all(&dir)?;

    let path = format!("{}/last_run", dir);
    fs::write(path, hash)?;
    Ok(())
}
