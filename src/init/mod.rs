use std::fs;
use anyhow::Result;
use crate::parser;
use crate::yaml;

pub fn run_init() -> Result<()> {
    for dir in ["bee/tasks", "bee/rules", "bee/pipelines", "bee/cache"] {
        fs::create_dir_all(dir)?;
    }

    let task_build_config = parser::Task {
        run: String::from("echo \"Building...!\""),
        depends_on: Some(vec![]),
    };

    yaml::writer::save_yaml("bee/tasks/build.yml", &task_build_config)?;
    println!("Created sample tasks/build.yml");

    let task_test_config = parser::Task {
        run: String::from("echo \"Testing...!\""),
        depends_on: Some(vec![String::from("build")]),
    };

    yaml::writer::save_yaml("bee/tasks/test.yml", &task_test_config)?;
    println!("Created sample tasks/test.yml");

    let pipeline_main_config = parser::Pipeline {
        tasks: vec![String::from("build"), String::from("test")],
    };

    yaml::writer::save_yaml("bee/pipelines/main.yml", &pipeline_main_config)?;
    println!("Created sample pipelines/main.yml");

    let main_config = parser::MainConfig {
        tasks: vec![String::from("build"), String::from("test")],
        rules: vec![],
        pipelines: vec![String::from("main")]
    };

    yaml::writer::save_yaml("bee/config.yml", &main_config)?;
    println!("Created main config file");

    Ok(())
}