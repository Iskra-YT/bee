use crate::file;
use crate::hash;
use crate::safety;
use crate::yaml;
use anyhow::{anyhow, Result};
use std::fs;

fn remove_registered(kind: &str, name: &str, dir: &str) -> Result<()> {
    safety::validate_name(kind, name)?;
    let path = safety::safe_path(dir, kind, name)?.with_extension("yml");
    if !path.exists() {
        return Err(anyhow!("{} '{}' not found", kind, name));
    }

    // Move the definition aside first: if the registry update fails, restore
    // it instead of leaving config.yml out of sync with the filesystem.
    let staged = path.with_extension("yml.bak");
    if staged.exists() {
        fs::remove_file(&staged)?;
    }
    fs::rename(&path, &staged)?;
    let updated = yaml::modify::remove_from_yaml_list(
        "bee/system/config.yml",
        &format!("{}s", kind),
        name,
    )
    .and_then(|_| {
        let config_hash = hash::hash_string(&file::get_file_content("bee/system/config.yml")?);
        hash::write_hash_file("bee/system/hash/config", &config_hash)?;
        Ok(())
    });
    match updated {
        Ok(()) => {
            fs::remove_file(&staged)?;
            println!("{} '{}' deleted", kind, name);
            Ok(())
        }
        Err(err) => {
            let _ = fs::rename(&staged, &path);
            Err(err)
        }
    }
}

pub fn delete_task(name: &String) -> Result<()> {
    remove_registered("task", name, "bee/tasks")
}

pub fn delete_pipeline(name: &String) -> Result<()> {
    remove_registered("pipeline", name, "bee/pipelines")
}

pub fn delete_rule(name: &String) -> Result<()> {
    remove_registered("rule", name, "bee/rules")
}
