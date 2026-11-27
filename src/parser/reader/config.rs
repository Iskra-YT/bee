use crate::parser::{
    Dependency, DepsConfig, Pipeline, PipelineConfig, Rule, RuleConfig, RuleTrigger, Task,
    TaskConfig,
};
use crate::{file, safety, yaml};
use anyhow::{anyhow, Context, Result};

fn validate_registry_names(kind: &str, names: &[String]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for name in names {
        safety::validate_name(kind, name)?;
        if !seen.insert(name) {
            return Err(anyhow!(
                "duplicate {} '{}' in bee/system/config.yml",
                kind,
                name
            ));
        }
    }
    Ok(())
}

fn registry_names(key: &str) -> Result<Vec<String>> {
    let yaml_content = file::get_file_content("./bee/system/config.yml")?;
    let root = yaml::reader::parse_yaml_file(&yaml_content)?;
    let names: Vec<String> = root
        .get(key)
        .and_then(|p| p.as_sequence())
        .ok_or_else(|| anyhow!("bee/system/config.yml is missing the '{}' list", key))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("invalid entry in '{}' list: expected a name", key))
        })
        .collect::<Result<Vec<_>>>()?;
    validate_registry_names(key.trim_end_matches('s'), &names)?;
    Ok(names)
}

pub fn read_pipelines_config() -> Result<Vec<PipelineConfig>> {
    Ok(registry_names("pipelines")?
        .into_iter()
        .map(|name| PipelineConfig { name })
        .collect())
}

pub fn read_tasks_config() -> Result<Vec<TaskConfig>> {
    Ok(registry_names("tasks")?
        .into_iter()
        .map(|name| TaskConfig { name })
        .collect())
}

pub fn read_rules_config() -> Result<Vec<RuleConfig>> {
    Ok(registry_names("rules")?
        .into_iter()
        .map(|name| RuleConfig { name })
        .collect())
}

pub fn read_pipelines() -> Result<Vec<Pipeline>> {
    let mut res = Vec::new();
    for config in read_pipelines_config()? {
        let path = format!("bee/pipelines/{}.yml", config.name);
        let content = file::get_file_content(&path)
            .with_context(|| format!("pipeline '{}' listed in config.yml", config.name))?;
        let pipeline = read_pipeline_from_string(&content)
            .with_context(|| format!("invalid pipeline file '{}'", path))?;
        if pipeline.name != config.name {
            return Err(anyhow!(
                "pipeline name '{}' in '{}' does not match registry name '{}'",
                pipeline.name,
                path,
                config.name
            ));
        }
        res.push(pipeline);
    }
    Ok(res)
}

pub fn read_pipeline_from_string(content: &str) -> Result<Pipeline> {
    let pipeline: Pipeline =
        serde_yaml::from_str(content).map_err(|e| anyhow!("invalid pipeline YAML: {}", e))?;
    safety::validate_name("pipeline", &pipeline.name)?;

    let mut seen = std::collections::HashSet::new();
    for task in &pipeline.tasks {
        safety::validate_name("task reference", task)?;
        if !seen.insert(task) {
            return Err(anyhow!(
                "pipeline '{}' lists task '{}' more than once",
                pipeline.name,
                task
            ));
        }
    }
    for sub in &pipeline.pipelines {
        safety::validate_name("subpipeline reference", sub)?;
    }
    if pipeline.tasks.is_empty() && pipeline.pipelines.is_empty() {
        return Err(anyhow!(
            "pipeline '{}' has no tasks or subpipelines",
            pipeline.name
        ));
    }
    Ok(pipeline)
}

pub fn read_pipeline(name: &str) -> Result<Pipeline> {
    safety::validate_name("pipeline", name)?;
    let pipeline_file = format!("bee/pipelines/{}.yml", name);
    let content = file::get_file_content(&pipeline_file)
        .with_context(|| format!("pipeline '{}' not found", name))?;
    read_pipeline_from_string(&content)
}

pub fn read_task_from_string(content: &str, name: &str) -> Result<Task> {
    safety::validate_name("task", name)?;
    let mut task: Task = serde_yaml::from_str(content)
        .map_err(|e| anyhow!("failed to parse task '{}': {}", name, e))?;
    if task.name.is_empty() {
        task.name = name.to_string();
    } else if task.name != name {
        return Err(anyhow!(
            "task name '{}' in bee/tasks/{}.yml does not match filename '{}'",
            task.name,
            name,
            name
        ));
    }
    task.validate()
        .map_err(|e| anyhow!("invalid task '{}': {}", name, e))?;
    Ok(task)
}

pub fn read_rules_from_string(content: &str) -> Result<Rule> {
    let rule: Rule =
        serde_yaml::from_str(content).map_err(|e| anyhow!("invalid rule YAML: {}", e))?;
    safety::validate_name("rule", &rule.name)?;
    safety::validate_name("task reference", &rule.task)?;

    for trigger in &rule.triggers {
        validate_trigger(&rule.name, trigger)?;
    }
    Ok(rule)
}

fn validate_trigger(rule_name: &str, trigger: &RuleTrigger) -> Result<()> {
    let missing = |field: &str| {
        anyhow!(
            "rule '{}': trigger '{}' requires a non-empty '{}'",
            rule_name,
            trigger_type(trigger),
            field
        )
    };

    match trigger {
        RuleTrigger::FileChange { paths }
        | RuleTrigger::GitChanged { paths }
        | RuleTrigger::ChecksumChanged { paths } => {
            if paths.is_empty() || paths.iter().any(|p| p.trim().is_empty()) {
                return Err(missing("paths"));
            }
        }
        RuleTrigger::EnvChanged { vars } => {
            if vars.is_empty() || vars.iter().any(|v| v.trim().is_empty()) {
                return Err(missing("vars"));
            }
        }
        RuleTrigger::TaskCompleted { task }
        | RuleTrigger::TaskFailed { task }
        | RuleTrigger::DependencyChanged { task }
        | RuleTrigger::ParentTaskCompleted { task }
        | RuleTrigger::ParentTaskFailed { task } => {
            safety::validate_name("task reference", task)?;
            if task.trim().is_empty() {
                return Err(missing("task"));
            }
        }
        RuleTrigger::DependencyMissing { path } => {
            if path.trim().is_empty() {
                return Err(missing("path"));
            }
        }
        RuleTrigger::Schedule { cron } => {
            if cron.trim().is_empty() {
                return Err(missing("cron"));
            }
            crate::schedule::validate_expression(cron)
                .map_err(|e| anyhow!("rule '{}': {}", rule_name, e))?;
        }
        RuleTrigger::SubtaskCompleted { subsystem, task }
        | RuleTrigger::SubtaskFailed { subsystem, task }
        | RuleTrigger::SubtaskChanged { subsystem, task } => {
            safety::validate_name("subsystem reference", subsystem)?;
            safety::validate_name("task reference", task)?;
        }
        RuleTrigger::SubsystemChanged { subsystem } => {
            safety::validate_name("subsystem reference", subsystem)?;
        }
        RuleTrigger::GitTag { pattern: _ } | RuleTrigger::Manual | RuleTrigger::Always => {}
    }
    Ok(())
}

fn trigger_type(trigger: &RuleTrigger) -> &'static str {
    match trigger {
        RuleTrigger::Manual => "manual",
        RuleTrigger::Always => "always",
        RuleTrigger::FileChange { .. } => "file_change",
        RuleTrigger::GitChanged { .. } => "git_changed",
        RuleTrigger::TaskCompleted { .. } => "task_completed",
        RuleTrigger::TaskFailed { .. } => "task_failed",
        RuleTrigger::DependencyChanged { .. } => "dependency_changed",
        RuleTrigger::EnvChanged { .. } => "env_changed",
        RuleTrigger::DependencyMissing { .. } => "dependency_missing",
        RuleTrigger::ChecksumChanged { .. } => "checksum_changed",
        RuleTrigger::Schedule { .. } => "schedule",
        RuleTrigger::GitTag { .. } => "git_tag",
        RuleTrigger::SubtaskCompleted { .. } => "subtask_completed",
        RuleTrigger::SubtaskFailed { .. } => "subtask_failed",
        RuleTrigger::SubtaskChanged { .. } => "subtask_changed",
        RuleTrigger::ParentTaskCompleted { .. } => "parent_task_completed",
        RuleTrigger::ParentTaskFailed { .. } => "parent_task_failed",
        RuleTrigger::SubsystemChanged { .. } => "subsystem_changed",
    }
}

pub fn read_rules() -> Result<Vec<Rule>> {
    let mut rules = Vec::new();
    for config in read_rules_config()? {
        let path = format!("bee/rules/{}.yml", config.name);
        let content = file::get_file_content(&path)
            .with_context(|| format!("rule '{}' listed in config.yml", config.name))?;
        let rule = read_rules_from_string(&content)
            .with_context(|| format!("invalid rule file '{}'", path))?;
        if rule.name != config.name {
            return Err(anyhow!(
                "rule name '{}' in '{}' does not match registry name '{}'",
                rule.name,
                path,
                config.name
            ));
        }
        rules.push(rule);
    }
    Ok(rules)
}

fn validate_deps(deps: &[Dependency]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for dep in deps {
        safety::validate_name("dependency", &dep.name)?;
        if !seen.insert(&dep.name) {
            return Err(anyhow!("duplicate dependency '{}' in deps.yml", dep.name));
        }
        match dep.dep_type.as_str() {
            "package" | "url" | "git" | "command" | "manager" => {}
            other => {
                return Err(anyhow!(
                    "dependency '{}': unknown type '{}'",
                    dep.name,
                    other
                ))
            }
        }
    }
    Ok(())
}

pub fn read_deps() -> Result<Vec<Dependency>> {
    let content = file::get_file_content("bee/system/deps.yml")?;
    let deps_config: DepsConfig = serde_yaml::from_str(&content)
        .map_err(|e| anyhow!("invalid bee/system/deps.yml: {}", e))?;
    validate_deps(&deps_config.dependencies)?;
    Ok(deps_config.dependencies)
}

pub fn read_deps_config() -> Result<Vec<Dependency>> {
    read_deps()
}
