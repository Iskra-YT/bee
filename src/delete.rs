use anyhow::Result;
use std::fs;
use crate::yaml;
use crate::file;
use crate::hash;

pub fn delete_task(name: &String) -> Result<()> {
    fs::remove_file(format!("bee/tasks/{}.yml", name))?;
    yaml::modify::remove_from_yaml_list(&String::from("bee/system/config.yml"), &String::from("tasks"), name)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    file::write_file_content(&String::from("bee/system/hash/config"), &config_hash)?;

    println!("[bee/info] Task '{}' deleted", name);
    Ok(())
}

pub fn delete_pipeline(name: &String) -> Result<()> {
    fs::remove_file(format!("bee/pipelines/{}.yml", name))?;
    yaml::modify::remove_from_yaml_list(&String::from("bee/system/config.yml"), &String::from("pipelines"), name)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    file::write_file_content(&String::from("bee/system/hash/config"), &config_hash)?;

    println!("[bee/info] Pipeline '{}' deleted", name);
    Ok(())
}

pub fn delete_rule(name: &String) -> Result<()> {
    fs::remove_file(format!("bee/rules/{}.yml", name))?;
    yaml::modify::remove_from_yaml_list(&String::from("bee/system/config.yml"), &String::from("rules"), name)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    file::write_file_content(&String::from("bee/system/hash/config"), &config_hash)?;

    println!("[bee/info] Rule '{}' deleted", name);
    Ok(())
}
