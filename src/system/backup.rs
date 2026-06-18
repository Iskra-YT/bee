use anyhow::Result;
use crate::{file, hash};
use crate::time;
use std::fs;
use std::path::Path;

const MAX_BACKUPS: usize = 20;

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn count_backup_weight() -> Result<usize> {
    let mut count = 0usize;
    for dir in ["bee/tasks", "bee/pipelines", "bee/rules"].iter() {
        let path = Path::new(dir);
        if path.exists() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

fn get_timestamp() -> String {
    time::get_timestamp_string()
        .replace(":", "-")
        .replace("T", "_")
        .split('.')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

fn get_backup_hash() -> Result<String> {
    let config_content = file::get_file_content(&String::from("bee/system/config.yml"))?;
    Ok(hash::hash_string(&config_content))
}

fn cleanup_old_backups() -> Result<()> {
    let mut manifest = read_manifest()?;
    let versions = manifest["versions"].members().collect::<Vec<_>>();
    if versions.len() <= MAX_BACKUPS {
        return Ok(());
    }

    let remove_count = versions.len() - MAX_BACKUPS;
    let to_remove: Vec<String> = versions.iter()
        .take(remove_count)
        .filter_map(|v| v["hash"].as_str().map(|s| s.to_string()))
        .collect();

    for hash_val in &to_remove {
        let path = format!("bee/system/backup/{}", hash_val);
        let _ = fs::remove_dir_all(&path);
    }

    let remaining: Vec<json::JsonValue> = versions.into_iter()
        .skip(remove_count)
        .cloned()
        .collect();

    let versions_array = json::JsonValue::Array(remaining);
    manifest["versions"] = versions_array;
    write_manifest(&manifest)?;

    Ok(())
}

fn read_manifest() -> Result<json::JsonValue> {
    let path = "bee/system/backup/.manifest.json";
    if !Path::new(path).exists() {
        let mut manifest = json::object! {};
        manifest["latest"] = json::JsonValue::Null;
        manifest["versions"] = json::JsonValue::Array(vec![]);
        return Ok(manifest);
    }
    let content = file::get_file_content(&String::from(path))?;
    Ok(json::parse(&content)?)
}

fn write_manifest(manifest: &json::JsonValue) -> Result<()> {
    file::write_file_content(
        &String::from("bee/system/backup/.manifest.json"),
        &json::stringify(manifest.clone()),
    )?;
    Ok(())
}

pub fn make_backup() -> Result<()> {
    let hash_val = get_backup_hash()?;
    let short_hash: String = hash_val.chars().take(16).collect();
    let timestamp = get_timestamp();
    let weight = count_backup_weight()?;

    let backup_dir = format!("bee/system/backup/{}", short_hash);
    let backup_path = Path::new(&backup_dir);

    if backup_path.exists() {
        println!("Backup with hash {} already exists, skipping", short_hash);
        return Ok(());
    }

    fs::create_dir_all(backup_path)?;

    let dirs_to_backup = ["bee/tasks", "bee/pipelines", "bee/rules"];
    for dir in &dirs_to_backup {
        let src = Path::new(dir);
        if src.exists() {
            let dst = backup_path.join(dir.strip_prefix("bee/").unwrap_or(dir));
            copy_dir_all(src, &dst)?;
        }
    }

    let system_src = Path::new("bee/system");
    let system_dst = backup_path.join("system");
    fs::create_dir_all(&system_dst)?;

    let system_files = ["config.yml", "init"];
    for f in &system_files {
        let src_path = system_src.join(f);
        if src_path.exists() {
            fs::copy(&src_path, system_dst.join(f))?;
        }
    }

    let mut meta = json::object! {};
    meta["date"] = json::JsonValue::String(timestamp.clone());
    meta["hash"] = json::JsonValue::String(hash_val);
    meta["weight"] = json::JsonValue::Number((weight as f64).into());
    file::write_file_content(
        &format!("{}/.meta.json", backup_dir),
        &json::stringify(meta),
    )?;

    let mut manifest = read_manifest()?;
    manifest["latest"] = json::JsonValue::String(short_hash.clone());

    let mut entry = json::object! {};
    entry["hash"] = json::JsonValue::String(short_hash.clone());
    entry["date"] = json::JsonValue::String(timestamp.clone());
    entry["weight"] = json::JsonValue::Number((weight as f64).into());

    let mut versions: Vec<json::JsonValue> = manifest["versions"].members().cloned().collect();
    versions.push(entry);
    manifest["versions"] = json::JsonValue::Array(versions);

    write_manifest(&manifest)?;
    cleanup_old_backups()?;

    println!("Backup created: {}", backup_dir);
    println!("  Hash: {}", short_hash);
    println!("  Date: {}", timestamp);
    println!("  Weight: {}", weight);

    Ok(())
}

pub fn list_backups() -> Result<()> {
    let manifest = read_manifest()?;
    let versions: Vec<json::JsonValue> = manifest["versions"].members().cloned().collect();

    if versions.is_empty() {
        println!("No backups found");
        return Ok(());
    }

    println!("{:<20} | {:<20} | weight", "date", "hash");
    println!("{}", "-".repeat(60));

    let mut sorted = versions.clone();
    sorted.sort_by(|a, b| {
        let a_date = a["date"].as_str().unwrap_or("");
        let b_date = b["date"].as_str().unwrap_or("");
        a_date.cmp(b_date)
    });

    for entry in &sorted {
        if entry.is_string() {
            let ts = entry.as_str().unwrap_or("?");
            println!("{:<20} | {:<20} | ?", ts, "(legacy)");
            continue;
        }
        let date = entry["date"].as_str().unwrap_or("?");
        let hash_val = entry["hash"].as_str().unwrap_or("?");
        let weight = entry["weight"].as_f64().unwrap_or(0.0) as u64;
        println!("{:<20} | {:<20} | {}", date, hash_val, weight);
    }

    Ok(())
}

pub fn restore(hash_val: &str) -> Result<()> {
    let backup_dir = format!("bee/system/backup/{}", hash_val);
    let backup_path = Path::new(&backup_dir);

    if !backup_path.exists() {
        eprintln!("Error: backup '{}' not found", hash_val);
        return Ok(());
    }

    let dirs_to_restore = ["tasks", "pipelines", "rules"];
    for dir in &dirs_to_restore {
        let src = backup_path.join(dir);
        let dst = Path::new("bee").join(dir);
        if src.exists() {
            let _ = fs::remove_dir_all(&dst);
            copy_dir_all(&src, &dst)?;
        }
    }

    let system_src = backup_path.join("system");
    if system_src.exists() {
        let system_dst = Path::new("bee/system");
        let system_files = ["config.yml", "init"];
        for f in &system_files {
            let src_path = system_src.join(f);
            if src_path.exists() {
                fs::copy(&src_path, system_dst.join(f))?;
            }
        }
    }

    let meta_path = backup_path.join(".meta.json");
    if meta_path.exists() {
        let meta_content = file::get_file_content(&meta_path.to_string_lossy().to_string())?;
        let meta = json::parse(&meta_content)?;
        let date = meta["date"].as_str().unwrap_or("?");
        println!("Restored from backup: {} (date: {}, weight: {})", hash_val, date, meta["weight"]);
    } else {
        println!("Restored from backup: {}", hash_val);
    }

    Ok(())
}
