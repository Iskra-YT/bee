use anyhow::Result;
use std::fs;

pub fn clean_cache() -> Result<()> {
    let cache_path = "bee/cache";
    if fs::metadata(cache_path).is_ok() {
        fs::remove_dir_all(cache_path)?;
        println!("Cache cleaned");
    } else {
        println!("Cache is empty");
    }
    Ok(())
}
