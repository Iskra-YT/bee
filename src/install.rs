pub mod url;
pub mod git;
pub mod package;
pub mod command;

use std::fs;
use anyhow::Result;
use crate::parser::{Dependency, DepsConfig};
use crate::{file, yaml, hash};

const DEPS_PATH: &str = "bee/system/deps.yml";
const DEPS_HASH_PATH: &str = "bee/system/hash/deps";

fn update_deps_hash() -> Result<()> {
    let deps_hash = hash::hash_string(&file::get_file_content(&String::from(DEPS_PATH))?);
    file::write_file_content(&String::from(DEPS_HASH_PATH), &deps_hash)?;
    Ok(())
}

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
        "url" => {
            let source = dep.source.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Dependency '{}' of type 'url' requires a source", dep.name))?;
            url::install(&dep.name, source, dep.version.as_deref())?;
        }
        "git" => {
            let source = dep.source.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Dependency '{}' of type 'git' requires a source", dep.name))?;
            git::install(&dep.name, source, dep.version.as_deref())?;
        }
        "package" => {
            let source = dep.source.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Dependency '{}' of type 'package' requires a source", dep.name))?;
            package::install(&dep.name, source, dep.command.as_deref())?;
        }
        "command" => {
            let cmd = dep.command.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Dependency '{}' of type 'command' requires a command", dep.name))?;
            command::install(&dep.name, cmd)?;
        }
        _ => return Err(anyhow::anyhow!("Unknown dependency type: {}", dep.dep_type)),
    }

    Ok(())
}

pub fn add_dep(name: &str, dep_type: &str, source: Option<&str>, version: Option<&str>, command: Option<&str>) -> Result<()> {
    let mut deps = read_deps()?;

    if deps.iter().any(|d| d.name == name) {
        return Err(anyhow::anyhow!("Dependency '{}' already exists", name));
    }

    match dep_type {
        "url" | "git" | "package" => {
            if source.is_none() {
                return Err(anyhow::anyhow!("Dependency type '{}' requires a source", dep_type));
            }
        }
        "command" => {
            if command.is_none() {
                return Err(anyhow::anyhow!("Dependency type 'command' requires a command"));
            }
        }
        _ => return Err(anyhow::anyhow!("Unknown dependency type: {}", dep_type)),
    }

    let new_dep = Dependency {
        name: name.to_string(),
        dep_type: dep_type.to_string(),
        source: source.map(|s| s.to_string()),
        version: version.map(|v| v.to_string()),
        command: command.map(|c| c.to_string()),
    };

    deps.push(new_dep);
    write_deps(&deps)?;
    update_deps_hash()?;

    println!("Added dependency '{}'", name);
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
    update_deps_hash()?;

    let deps_dir = format!("bee/deps/{}", name);
    if std::path::Path::new(&deps_dir).exists() {
        fs::remove_dir_all(&deps_dir)?;
        println!("Removed installed dependency from {}", deps_dir);
    }

    println!("Removed dependency '{}'", name);
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
        let mut line = format!("  {} (type: {}", dep.name, dep.dep_type);
        if let Some(ref s) = dep.source {
            line.push_str(&format!(", source: {}", s));
        }
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
    if !std::path::Path::new(DEPS_PATH).exists() {
        return Ok(vec![]);
    }

    let content = file::get_file_content(&String::from(DEPS_PATH))?;
    let deps_config: DepsConfig = serde_yaml::from_str(&content)?;
    Ok(deps_config.dependencies)
}

fn write_deps(deps: &[Dependency]) -> Result<()> {
    let deps_config = DepsConfig {
        dependencies: deps.to_vec(),
    };
    yaml::writer::save_yaml(DEPS_PATH, &deps_config)?;
    Ok(())
}
