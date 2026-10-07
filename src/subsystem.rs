use anyhow::{anyhow, Result};
use crate::{file, parser, yaml};
use std::fs;
use std::path::{Path, PathBuf};

fn read_config(path: &Path) -> Result<parser::SubsystemsConfig> {
    if !path.exists() {
        return Ok(parser::SubsystemsConfig::default());
    }
    let content = file::get_file_content(&path.to_string_lossy().to_string())?;
    verify_config_hash(path, &content);
    Ok(serde_yaml::from_str(&content).unwrap_or_default())
}

fn verify_config_hash(path: &Path, content: &str) {
    let hash_path = match path.parent() {
        Some(p) => p.join("hash/subsystems"),
        None => return,
    };
    if !hash_path.exists() {
        return;
    }
    if let Ok(expected) = fs::read_to_string(&hash_path) {
        let actual = crate::hash::hash_string(content);
        if actual != expected.trim() {
            eprintln!("Warning: subsystems registry hash mismatch at '{}'", path.display());
        }
    }
}

fn update_subsystems_hash() -> Result<()> {
    let content = file::get_file_content(&String::from("bee/system/subsystems.yml"))?;
    let h = crate::hash::hash_string(&content);
    crate::hash::write_hash_file("bee/system/hash/subsystems", &h)?;
    Ok(())
}

pub fn deps_owner_dir() -> Option<PathBuf> {
    let mut dir = file::resolve_parent_dir()?;
    loop {
        if dir.join("bee/system/deps.yml").exists() {
            return Some(dir);
        }
        let link = dir.join("bee/system/parent");
        let Ok(content) = fs::read_to_string(&link) else {
            return None;
        };
        let up = Path::new(content.trim()).to_path_buf();
        let next = if up.is_absolute() { up } else { dir.join(up) };
        dir = fs::canonicalize(&next).ok()?;
    }
}

fn parent_config() -> Result<Option<(PathBuf, parser::SubsystemsConfig)>> {
    if !file::is_subsystem() {
        return Ok(None);
    }
    let parent = file::read_parent_link().ok_or_else(|| anyhow!("subsystem parent link is missing"))?;
    let parent_abs = fs::canonicalize(&parent)
        .map_err(|_| anyhow!("cannot resolve parent system at '{}'", parent.to_string_lossy()))?;
    let path = parent_abs.join("bee/system/subsystems.yml");
    let config = read_config(&path)?;
    Ok(Some((parent_abs, config)))
}

pub fn resolve_subsystem_dir(name: &str) -> Option<PathBuf> {
    let own = read_config(Path::new("bee/system/subsystems.yml")).ok()?;
    if let Some(s) = own.subsystems.iter().find(|s| s.name == name) {
        let abs = fs::canonicalize(&s.path).ok()?;
        return Some(abs);
    }

    if let Ok(Some((parent_abs, pcfg))) = parent_config() {
        let s = pcfg.subsystems.iter().find(|s| s.name == name)?;
        let abs = fs::canonicalize(parent_abs.join(&s.path)).ok()?;
        return Some(abs);
    }

    None
}

pub fn subsystem_run_task(subsystem: &str, task: &str) -> Result<()> {
    let Some(dir) = resolve_subsystem_dir(subsystem) else {
        eprintln!("Error: subsystem '{}' not found", subsystem);
        std::process::exit(1);
    };

    let exe = std::env::current_exe()?;
    let status = std::process::Command::new(&exe)
        .current_dir(&dir)
        .args(["task", "run", task])
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

pub fn subsystem_run_pipeline(subsystem: &str, pipeline: &str) -> Result<()> {
    let Some(dir) = resolve_subsystem_dir(subsystem) else {
        eprintln!("Error: subsystem '{}' not found", subsystem);
        std::process::exit(1);
    };

    let exe = std::env::current_exe()?;
    let status = std::process::Command::new(&exe)
        .current_dir(&dir)
        .args(["pipeline", "run", pipeline])
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

pub fn subsystem_remove(name: &str, delete: bool) -> Result<()> {
    if !file::check_bee_directory() {
        eprintln!("Error: run bee init first");
        return Ok(());
    }

    let path = Path::new("bee/system/subsystems.yml");
    let mut config = read_config(path)?;
    let Some(index) = config.subsystems.iter().position(|s| s.name == name) else {
        eprintln!("Error: subsystem '{}' not registered", name);
        return Ok(());
    };

    let subsystem = config.subsystems.remove(index);
    yaml::writer::save_yaml(path.to_str().unwrap_or("bee/system/subsystems.yml"), &config)?;
    update_subsystems_hash()?;
    println!("Subsystem '{}' removed from registry", name);

    if delete {
        let cwd = std::env::current_dir()?;
        let dir = cwd.join(&subsystem.path);
        match fs::canonicalize(&dir) {
            Ok(abs) => {
                fs::remove_dir_all(&abs)?;
                println!("  Deleted {}", abs.display());
            }
            Err(_) => {
                println!("  Warning: directory '{}' does not exist", subsystem.path);
            }
        }
    }
    Ok(())
}

pub fn subsystem_doctor() -> Result<()> {
    if !file::check_bee_directory() {
        eprintln!("Error: run bee init first");
        return Ok(());
    }

    let path = Path::new("bee/system/subsystems.yml");
    if !path.exists() {
        println!("No subsystems registered. Nothing to check.");
        return Ok(());
    }

    let config = read_config(path)?;
    let cwd = std::env::current_dir()?;
    println!("Checking {} registered subsystem(s)...", config.subsystems.len());

    let mut issues = 0usize;
    for s in &config.subsystems {
        let mut problems: Vec<String> = vec![];
        let dir = cwd.join(&s.path);

        match fs::canonicalize(&dir) {
            Ok(abs) => {
                if !abs.join("bee/system/init").exists() {
                    problems.push("not a valid bee system (missing bee/system/init)".into());
                } else {
                    let expected = file::relative_path(&abs, &cwd);
                    let link = abs.join("bee/system/parent");
                    match fs::read_to_string(&link) {
                        Ok(content) => {
                            if Path::new(content.trim()) != expected {
                                problems.push(format!(
                                    "parent link '{}' does not point here (expected '{}')",
                                    content.trim(),
                                    expected.to_string_lossy()
                                ));
                            }
                        }
                        Err(_) => problems.push("missing bee/system/parent".into()),
                    }
                }
            }
            Err(_) => problems.push(format!("directory '{}' does not exist", s.path)),
        }

        if problems.is_empty() {
            println!("  [ok]   {} (path: {})", s.name, s.path);
        } else {
            issues += problems.len();
            println!("  [!!]   {} (path: {})", s.name, s.path);
            for p in &problems {
                println!("         - {}", p);
            }
        }
    }

    if issues == 0 {
        println!("All subsystems OK");
    } else {
        println!("Found {} issue(s). Fix with 'bee subsystem remove <name>' then re-run 'bee init'.", issues);
    }
    Ok(())
}

pub fn subsystem_deps_list() -> Result<()> {
    let owner = deps_owner_dir().ok_or_else(|| anyhow!("not inside a subsystem"))?;
    let deps_path = owner.join("bee/system/deps.yml");
    let content = file::get_file_content(&deps_path.to_string_lossy().to_string())?;
    let deps: parser::DepsConfig = serde_yaml::from_str(&content).unwrap_or_default();
    if deps.dependencies.is_empty() {
        println!("Parent system defines no dependencies");
        return Ok(());
    }
    println!("Dependencies inherited from parent '{}':", owner.display());
    for d in &deps.dependencies {
        let installed = owner.join("bee/deps").join(&d.name).exists();
        println!("  - {} (type: {}, installed: {})", d.name, d.dep_type, if installed { "yes" } else { "no" });
    }
    println!("Reference them from subsystem tasks via $BEE_DEPS_PATH (read-only; install commands are not available in subsystems).");
    Ok(())
}

pub fn subsystem_deps_path(name: &str) -> Result<()> {
    let owner = deps_owner_dir().ok_or_else(|| anyhow!("not inside a subsystem"))?;
    let deps_path = owner.join("bee/system/deps.yml");
    let content = file::get_file_content(&deps_path.to_string_lossy().to_string())?;
    let deps: parser::DepsConfig = serde_yaml::from_str(&content).unwrap_or_default();
    match deps.dependencies.iter().find(|d| d.name == name) {
        Some(d) => println!("{}", owner.join("bee/deps").join(&d.name).display()),
        None => eprintln!("Error: dependency '{}' not defined in parent system", name),
    }
    Ok(())
}

pub fn subsystem_list() -> Result<()> {
    if let Some((parent_abs, config)) = parent_config()? {
        println!("Subsystems of parent system at '{}':", parent_abs.display());
        if config.subsystems.is_empty() {
            println!("  (none)");
            return Ok(());
        }
        let cwd = std::env::current_dir()?;
        let rel = file::relative_path(&parent_abs, &cwd).to_string_lossy().to_string();
        for s in &config.subsystems {
            let mark = if s.path == rel { " (current)" } else { "" };
            println!("  - {} (path: {}, date: {}){}", s.name, s.path, s.date, mark);
        }
        return Ok(());
    }

    if !file::check_bee_directory() {
        eprintln!("Error: run bee init first");
        return Ok(());
    }

    let config = read_config(Path::new("bee/system/subsystems.yml"))?;
    if config.subsystems.is_empty() {
        println!("No subsystems registered in this project");
        return Ok(());
    }
    println!("Subsystems registered in this project:");
    for s in &config.subsystems {
        println!("  - {} (path: {}, date: {})", s.name, s.path, s.date);
    }
    Ok(())
}

pub fn subsystem_status() -> Result<()> {
    if let Some((parent_abs, config)) = parent_config()? {
        println!("Bee subsystem status");
        println!("  Parent system: {}", parent_abs.display());
        if let Some(parent) = file::read_parent_link() {
            println!("  Parent link:   {}", parent.to_string_lossy());
        }
        let cwd = std::env::current_dir()?;
        let rel = file::relative_path(&parent_abs, &cwd).to_string_lossy().to_string();
        match config.subsystems.iter().find(|s| s.path == rel) {
            Some(s) => {
                println!("  Name:   {}", s.name);
                println!("  Path:   {}", s.path);
                println!("  Date:   {}", s.date);
                println!("  Hash:   {}", s.hash);
            }
            None => println!("  (not found in parent registry)"),
        }
        return Ok(());
    }

    if !file::check_bee_directory() {
        eprintln!("Error: run bee init first");
        return Ok(());
    }

    let config = read_config(Path::new("bee/system/subsystems.yml"))?;
    if config.subsystems.is_empty() {
        println!("This project has no subsystems registered");
    } else {
        println!("This project has {} subsystem(s). Use 'bee subsystem list' to see them.", config.subsystems.len());
    }
    Ok(())
}
