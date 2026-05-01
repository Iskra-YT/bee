use anyhow::Result;
use crate::parser;
use crate::yaml;


pub fn create_task(name: &String) -> Result<()> {
    let task_config = parser::Task {
        run: format!("echo \"Running {}...\"", name),
        depends_on: None,
    };

    yaml::writer::save_yaml(&format!("bee/tasks/{}.yml", name), &task_config)?;
    yaml::modify::append_to_yaml_list(&String::from("bee/config.yml"), &String::from("tasks"), name)?;

    Ok(())
}

pub fn create_pipeline(name: &String) -> Result<()> {
    let pipeline_config = parser::Pipeline {
        name: name.clone(),
        tasks: vec![],
    };

    yaml::writer::save_yaml(&format!("bee/pipelines/{}.yml", name), &pipeline_config)?;
    yaml::modify::append_to_yaml_list(&String::from("bee/config.yml"), &String::from("pipelines"), name)?;

    Ok(())
}

pub fn create_rule(name: &String) -> Result<()> {
    let rule_config = parser::Rule {
        task: name.clone(),
        actions: vec![parser::RuleAction {
            input: String::from("input_pattern"),
            output: String::from("output_pattern"),
        }]
    };

    yaml::writer::save_yaml(&format!("bee/rules/{}.yml", name), &rule_config)?;
    yaml::modify::append_to_yaml_list(&String::from("bee/config.yml"), &String::from("rules"), name)?;

    Ok(())
}