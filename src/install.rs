pub mod url;
pub mod git;
pub mod package;

use std::fs;
use anyhow::Result;
use crate::parser::{Dependency, DepsConfig};
use crate::{file, yaml};

pub fn install_all() -> Result<()> {
    let deps = read_deps()?;

    if deps.is_empty() {
        println!("No dependencies to install.");
        return Ok(());
    }

    println!("Installing {} dependencies...", deps.len());

    for dep in &deps {
        install_one(dep)?;
    }

    println!("All dependencies installed.");
    Ok(())
}

pub fn install_one(dep: &Dependency) -> Result<()> {
    println!("Installing {} (type: {})...", dep.name, dep.dep_type);

    match dep.dep_type.as_str() {
        "url" => url::install(&dep.name, &dep.source, dep.version.as_deref())?,
        "git" => git::install(&dep.name, &dep.source, dep.version.as_deref())?,
        "package" => package::install(&dep.name, &dep.source, dep.command.as_deref())?,
        _ => return Err(anyhow::anyhow!("Unknown dependency type: {}", dep.dep_type)),
    }

    Ok(())
}

pub fn add_dep(name: &str, dep_type: &str, source: &str, version: Option<&str>, command: Option<&str>) -> Result<()> {
    let mut deps = read_deps()?;

    if deps.iter().any(|d| d.name == name) {
        return Err(anyhow::anyhow!("Dependency '{}' already exists", name));
    }

    let new_dep = Dependency {
        name: name.to_string(),
        dep_type: dep_type.to_string(),
        source: source.to_string(),
        version: version.map(|v| v.to_string()),
        command: command.map(|c| c.to_string()),
    };

    deps.push(new_dep);
    write_deps(&deps)?;

    println!("Added dependency '{}' to deps.yml", name);
    Ok(())
}

pub fn remove_dep(name: &str) -> Result<()> {
    let mut deps = read_deps()?;
    let initial_len = deps.len();

    deps.retain(|d| d.name != name);

    if deps.len() == initial_len {
        return Err(anyhow::anyhow!("Dependency '{}' not found", name));
    }

    write_deps(&deps)?;

    let deps_dir = format!("bee/deps/{}", name);
    if std::path::Path::new(&deps_dir).exists() {
        fs::remove_dir_all(&deps_dir)?;
        println!("Removed installed dependency from {}", deps_dir);
    }

    println!("Removed dependency '{}' from deps.yml", name);
    Ok(())
}

pub fn list_deps() -> Result<()> {
    let deps = read_deps()?;

    if deps.is_empty() {
        println!("No dependencies defined.");
        return Ok(());
    }

    println!("Dependencies:");
    for dep in &deps {
        let mut line = format!("  {} (type: {}, source: {}", dep.name, dep.dep_type, dep.source);
        if let Some(ref v) = dep.version {
            line.push_str(&format!(", version: {}", v));
        }
        if let Some(ref c) = dep.command {
            line.push_str(&format!(", command: {}", c));
        }
        line.push(')');
        println!("{}", line);
    }

    Ok(())
}

fn read_deps() -> Result<Vec<Dependency>> {
    if !std::path::Path::new("bee/deps.yml").exists() {
        return Ok(vec![]);
    }

    let content = file::get_file_content(&String::from("bee/deps.yml"))?;
    let deps_config: DepsConfig = serde_yaml::from_str(&content)?;
    Ok(deps_config.dependencies)
}

fn write_deps(deps: &[Dependency]) -> Result<()> {
    let deps_config = DepsConfig {
        dependencies: deps.to_vec(),
    };
    yaml::writer::save_yaml("bee/deps.yml", &deps_config)?;
    Ok(())
}
