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
