pub mod url;
pub mod git;
pub mod package;
pub mod command;

use std::fs;
use std::process::Command;
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

fn run_install_script(name: &str) -> Result<()> {
    let script_path = std::fs::canonicalize(format!("bee/scripts/install/{}.sh", name))
        .map_err(|_| anyhow::anyhow!("Install script not found for {}", name))?;

    println!("  Running install script for {}...", name);
    let status = Command::new("sh")
        .arg(&script_path)
        .current_dir(format!("bee/deps/{}", name))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow::anyhow!("install script failed with exit code {} for {}", code, name));
    }

    println!("  Install script completed for {}", name);
    Ok(())
}

pub fn install_all() -> Result<()> {
    install_deps(false)
}

pub fn update_all() -> Result<()> {
    install_deps(true)
}

fn install_deps(update: bool) -> Result<()> {
    let deps = read_deps()?;

    if deps.is_empty() {
        println!("No dependencies to install.");
        return Ok(());
    }

    let action = if update { "Updating" } else { "Installing" };
    println!("{} {} dependencies...", action, deps.len());

    for dep in &deps {
        install_one(dep, update)?;
    }

    println!("All dependencies {}.", if update { "updated" } else { "installed" });
    Ok(())
}

pub fn install_one(dep: &Dependency, update: bool) -> Result<()> {
    let action = if update { "Updating" } else { "Installing" };
    println!("{} {} (type: {})...", action, dep.name, dep.dep_type);

    match dep.dep_type.as_str() {
        "url" => {
            let source = dep.source.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Dependency '{}' of type 'url' requires a source", dep.name))?;
            let installed = url::install(&dep.name, source, dep.version.as_deref(), update)?;
            if installed {
                run_install_script(&dep.name)?;
            }
        }
        "git" => {
            let source = dep.source.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Dependency '{}' of type 'git' requires a source", dep.name))?;
            let installed = git::install(&dep.name, source, dep.version.as_deref(), update)?;
            if installed {
                run_install_script(&dep.name)?;
            }
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

pub fn add_dep(name: &str, dep_type: &str, source: Option<&str>, version: Option<&str>, command: Option<&str>, install_script: Option<&str>) -> Result<()> {
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

    if let Some(script) = install_script {
    let script_path = format!("bee/scripts/install/{}.sh", name);
        fs::create_dir_all("bee/scripts/install")?;
        file::write_file_content(&String::from(&script_path), &script.to_string())?;
        fs::set_permissions(&script_path, std::os::unix::fs::PermissionsExt::from_mode(0o755))?;
    }

    let new_dep = Dependency {
        name: name.to_string(),
        dep_type: dep_type.to_string(),
        source: source.map(|s| s.to_string()),
        version: version.map(|v| v.to_string()),
        command: command.map(|c| c.to_string()),
        install_script: install_script.is_some(),
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

    let script_path = format!("bee/scripts/install/{}.sh", name);
    if std::path::Path::new(&script_path).exists() {
        fs::remove_file(&script_path)?;
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
        if dep.install_script {
            line.push_str(", install_script: yes");
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
