use std::fs;

pub fn get_state(key: &str) -> Option<String> {
    let path = format!("bee/cache/triggers/{}", key);
    fs::read_to_string(path).ok()
}

pub fn save_state(key: &str, value: &str) -> anyhow::Result<()> {
    fs::create_dir_all("bee/cache/triggers")?;
    fs::write(format!("bee/cache/triggers/{}", key), value)?;
    Ok(())
}

pub fn clear_state(key: &str) -> anyhow::Result<()> {
    let path = format!("bee/cache/triggers/{}", key);
    if std::path::Path::new(&path).exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn clear_all() -> anyhow::Result<()> {
    let dir = "bee/cache/triggers";
    if std::path::Path::new(dir).exists() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}
