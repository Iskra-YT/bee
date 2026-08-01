use crate::hash;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn get_file_content(path: &String) -> Result<String> {
    return Ok(fs::read_to_string(path)?);
}

pub fn write_file_content(path: &String, content: &String) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

pub fn check_bee_directory() -> bool {
    is_valid_system_at(Path::new("."))
}

pub fn is_subsystem() -> bool {
    Path::new("bee/system/parent").exists()
}

fn is_valid_system_at(rel: &Path) -> bool {
    let init_path = rel.join("bee/system/init");
    let init_hash_path = rel.join("bee/system/hash/init");
    if !init_path.exists() || !init_hash_path.exists() {
        return false;
    }
    let init_content = fs::read_to_string(&init_path).unwrap_or_default();
    let init_hash = fs::read_to_string(&init_hash_path).unwrap_or_default();
    hash::hash_string(init_content.as_str()) == init_hash
}

pub fn find_parent_system() -> Option<std::path::PathBuf> {
    let mut up = Path::new("..").to_path_buf();
    for _ in 0..64 {
        if is_valid_system_at(&up) {
            return Some(up);
        }
        up.push("..");
    }
    None
}

pub fn read_parent_link() -> Option<std::path::PathBuf> {
    if !is_subsystem() {
        return None;
    }
    match fs::read_to_string("bee/system/parent") {
        Ok(content) => Some(Path::new(content.trim()).to_path_buf()),
        Err(_) => None,
    }
}

pub fn resolve_parent_dir() -> Option<std::path::PathBuf> {
    let parent = read_parent_link()?;
    fs::canonicalize(&parent).ok()
}

pub fn relative_path(from: &Path, to: &Path) -> std::path::PathBuf {
    use std::path::Component;
    let from_c: Vec<Component> = from.components().collect();
    let to_c: Vec<Component> = to.components().collect();
    let mut common = 0;
    for (a, b) in from_c.iter().zip(to_c.iter()) {
        if a == b {
            common += 1;
        } else {
            break;
        }
    }

    let mut result = std::path::PathBuf::new();
    for _ in common..from_c.len() {
        result.push("..");
    }
    for component in &to_c[common..] {
        result.push(component.as_os_str());
    }
    result
}
