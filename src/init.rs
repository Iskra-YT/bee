use std::fs;
use anyhow::Result;
use crate::parser;
use crate::yaml;
use crate::hash;
use crate::time;
use crate::file;

pub fn run_init() -> Result<()> {
    if file::check_bee_directory() {
        println!("bee is already initialized in this directory");
        return Ok(());
    }
    let _ = fs::remove_dir_all("bee");
    for dir in ["bee/tasks", "bee/rules", "bee/pipelines", "bee/cache", "bee/system", "bee/system/hash"].iter() {
        fs::create_dir_all(dir)?;
    }

    let task_build_config = parser::Task {
        name: String::from("build"),
        run: String::from("echo \"Building...!\""),
        depends_on: Some(vec![]),
    };

    yaml::writer::save_yaml("bee/tasks/build.yml", &task_build_config)?;
    println!("  Created tasks/build.yml");

    let task_test_config = parser::Task {
        name: String::from("test"),
        run: String::from("echo \"Testing...!\""),
        depends_on: Some(vec![String::from("build")]),
    };

    yaml::writer::save_yaml("bee/tasks/test.yml", &task_test_config)?;
    println!("  Created tasks/test.yml");

    let pipeline_main_config = parser::Pipeline {
        name: String::from("main"),
        tasks: vec![String::from("build"), String::from("test")],
    };

    yaml::writer::save_yaml("bee/pipelines/main.yml", &pipeline_main_config)?;
    println!("  Created pipelines/main.yml");

    let main_config = parser::MainConfig {
        tasks: vec![String::from("build"), String::from("test")],
        rules: vec![],
        pipelines: vec![String::from("main")]
    };

    yaml::writer::save_yaml("bee/system/config.yml", &main_config)?;
    println!("  Created system/config.yml");

    let init_proof: String = hash::hash_string(time::get_timestamp_string().as_str());
    file::write_file_content(&String::from("bee/system/init"), &init_proof)?;

    let init_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/init"))?);
    file::write_file_content(&String::from("bee/system/hash/init"), &init_hash)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    file::write_file_content(&String::from("bee/system/hash/config"), &config_hash)?;

    println!("Initialization complete. Hash: {}", init_proof.chars().take(8).collect::<String>());
    Ok(())
}