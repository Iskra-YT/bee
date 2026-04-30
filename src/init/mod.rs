use std::fs;
use anyhow::Result;

pub fn run_init() -> Result<()> {
    for dir in ["bee/tasks", "bee/rules", "bee/pipelines"] {
        fs::create_dir_all(dir)?;
    }
    
    Ok(())
}