use crate::time;
use crate::{file, hash, safety};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_BACKUPS: usize = 20;
const BACKUP_ROOT: &str = "bee/system/backup";
const RESTORE_OLD: &str = "bee/.restore-old";

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_symlink() {
            return Err(anyhow!(
                "backup contains a symlink ('{}'); refusing to restore it",
                src_path.display()
            ));
        }
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if ty.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn count_backup_weight() -> Result<usize> {
    let mut count = 0usize;
    for dir in ["bee/tasks", "bee/pipelines", "bee/rules"] {
        let path = Path::new(dir);
        if path.exists() {
            for entry in fs::read_dir(path)? {
                if entry?.file_type()?.is_file() {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

fn get_timestamp() -> String {
    time::get_timestamp_string()
        .replace(':', "-")
        .replace('T', "_")
        .split('.')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Hash the actual content of everything a backup restores. Hashing only
/// `config.yml` missed every edit to a task, rule or pipeline body.
fn get_backup_hash() -> Result<String> {
    let mut entries: Vec<String> = Vec::new();
    for dir in ["bee/tasks", "bee/pipelines", "bee/rules"] {
        let path = Path::new(dir);
        if !path.exists() {
            continue;
        }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                entries.push(format!("{}:{}", entry.path().display(), hash::hash_file(
                    &entry.path().to_string_lossy()
                )?));
            }
        }
    }
    entries.sort();
    entries.push(format!(
        "config.yml:{}",
        hash::hash_string(&file::get_file_content("bee/system/config.yml")?)
    ));
    Ok(hash::hash_string(&entries.join("\x1f")))
}

fn backup_path(hash_val: &str) -> Result<PathBuf> {
    safety::validate_hash_token("backup hash", hash_val)?;
    Ok(Path::new(BACKUP_ROOT).join(hash_val))
}

fn cleanup_old_backups() -> Result<()> {
    let mut manifest = read_manifest()?;
    let versions = manifest["versions"].members().collect::<Vec<_>>();
    if versions.len() <= MAX_BACKUPS {
        return Ok(());
    }

    let remove_count = versions.len() - MAX_BACKUPS;
    let mut to_remove = Vec::new();
    for version in versions.iter().take(remove_count) {
        let value = version["hash"]
            .as_str()
            .ok_or_else(|| anyhow!("backup manifest has an entry without a hash"))?;
        safety::validate_hash_token("backup hash in manifest", value)?;
        to_remove.push(value.to_string());
    }

    for hash_val in &to_remove {
        fs::remove_dir_all(backup_path(hash_val)?)?;
    }

    let remaining: Vec<json::JsonValue> = versions
        .into_iter()
        .skip(remove_count)
        .cloned()
        .collect();
    manifest["versions"] = json::JsonValue::Array(remaining);
    write_manifest(&manifest)
}

fn read_manifest() -> Result<json::JsonValue> {
    let path = format!("{}/.manifest.json", BACKUP_ROOT);
    if !Path::new(&path).exists() {
        let mut manifest = json::object! {};
        manifest["latest"] = json::JsonValue::Null;
        manifest["versions"] = json::JsonValue::Array(vec![]);
        return Ok(manifest);
    }
    let content = file::get_file_content(&path)?;
    Ok(json::parse(&content)?)
}

fn write_manifest(manifest: &json::JsonValue) -> Result<()> {
    safety::atomic_write(
        format!("{}/.manifest.json", BACKUP_ROOT),
        json::stringify(manifest.clone()).as_bytes(),
    )
}

pub fn make_backup() -> Result<()> {
    let hash_val = get_backup_hash()?;
    let short_hash: String = hash_val.chars().take(16).collect();
    let timestamp = get_timestamp();
    let weight = count_backup_weight()?;
    let backup_path = backup_path(&short_hash)?;

    if backup_path.exists() {
        println!("Identical configuration already backed up as {}", short_hash);
        return Ok(());
    }

    fs::create_dir_all(&backup_path)?;
    for dir in ["bee/tasks", "bee/pipelines", "bee/rules"] {
        let src = Path::new(dir);
        if src.exists() {
            copy_dir_all(src, &backup_path.join(dir.strip_prefix("bee/").unwrap_or(dir)))?;
        }
    }

    let system_dst = backup_path.join("system");
    fs::create_dir_all(&system_dst)?;
    for f in ["config.yml"] {
        let src_path = Path::new("bee/system").join(f);
        if src_path.exists() {
            fs::copy(&src_path, system_dst.join(f))?;
        }
    }

    let mut meta = json::object! {};
    meta["date"] = json::JsonValue::String(timestamp.clone());
    meta["hash"] = json::JsonValue::String(hash_val.clone());
    meta["weight"] = json::JsonValue::Number((weight as f64).into());
    safety::atomic_write(
        backup_path.join(".meta.json"),
        json::stringify(meta).as_bytes(),
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

    println!("Backup created: {}", backup_path.display());
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
    let mut sorted = versions;
    sorted.sort_by(|a, b| {
        a["date"]
            .as_str()
            .unwrap_or("")
            .cmp(b["date"].as_str().unwrap_or(""))
    });
    for entry in &sorted {
        let date = entry["date"].as_str().unwrap_or("?");
        let hash_val = entry["hash"].as_str().unwrap_or("?");
        let weight = entry["weight"].as_f64().unwrap_or(0.0) as u64;
        println!("{:<20} | {:<20} | {}", date, hash_val, weight);
    }
    Ok(())
}

fn rollback_restore(dirs: &[&str], config_before: Option<&str>) -> Result<()> {
    let old_root = Path::new(RESTORE_OLD);
    for dir in dirs.iter().rev() {
        let dst = Path::new("bee").join(dir);
        let backup = old_root.join(dir);
        if dst.exists() {
            fs::remove_dir_all(&dst)?;
        }
        if backup.exists() {
            fs::rename(&backup, &dst)?;
        }
    }
    if let Some(config) = config_before {
        safety::atomic_write("bee/system/config.yml", config.as_bytes())?;
    }
    if old_root.exists() {
        fs::remove_dir_all(old_root)?;
    }
    Ok(())
}

pub fn restore(hash_val: &str) -> Result<()> {
    let backup_path = backup_path(hash_val)?;
    if !backup_path.exists() {
        return Err(anyhow!("backup '{}' not found", hash_val));
    }

    let dirs = ["tasks", "pipelines", "rules"];
    for dir in &dirs {
        let src = backup_path.join(dir);
        if !src.is_dir() {
            return Err(anyhow!(
                "backup '{}' is incomplete: missing '{}' directory",
                hash_val,
                dir
            ));
        }
    }
    let config_src = backup_path.join("system/config.yml");
    if !config_src.is_file() {
        return Err(anyhow!(
            "backup '{}' is incomplete: missing system/config.yml",
            hash_val
        ));
    }
    let new_config = file::get_file_content(&config_src.to_string_lossy())?;
    let old_root = Path::new(RESTORE_OLD);
    if old_root.exists() {
        return Err(anyhow!(
            "another restore is in progress ({} exists); inspect it before retrying",
            RESTORE_OLD
        ));
    }
    fs::create_dir_all(old_root)?;

    let config_before = file::get_file_content("bee/system/config.yml").ok();
    let result = (|| -> Result<()> {
        for dir in &dirs {
            let dst = Path::new("bee").join(dir);
            if dst.exists() {
                fs::rename(&dst, old_root.join(dir))
                    .with_context(|| format!("could not set aside bee/{}", dir))?;
            }
            copy_dir_all(&backup_path.join(dir), &dst)
                .with_context(|| format!("could not restore bee/{}", dir))?;
        }
        safety::atomic_write("bee/system/config.yml", new_config.as_bytes())?;
        hash::write_hash_file("bee/system/hash/config", &new_config)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            fs::remove_dir_all(old_root)?;
            let meta_path = backup_path.join(".meta.json");
            let meta = file::get_file_content(&meta_path.to_string_lossy())
                .ok()
                .and_then(|content| json::parse(&content).ok());
            if let Some(meta) = meta {
                println!(
                    "Restored from backup: {} (date: {}, weight: {})",
                    hash_val,
                    meta["date"].as_str().unwrap_or("?"),
                    meta["weight"]
                );
            } else {
                println!("Restored from backup: {}", hash_val);
            }
            Ok(())
        }
        Err(err) => {
            if let Err(rollback_err) = rollback_restore(&dirs, config_before.as_deref()) {
                eprintln!("Warning: rollback failed: {}", rollback_err);
            }
            Err(err)
        }
    }
}
