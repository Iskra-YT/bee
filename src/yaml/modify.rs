use crate::file;
use anyhow::{anyhow, Result};
use serde_yaml::Value;

pub fn append_to_yaml_list(path: &str, list_name: &str, new_value: &str) -> Result<()> {
    let content = file::get_file_content(path)?;
    let mut yaml: Value = serde_yaml::from_str(&content)?;
    let root = yaml
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("{} is not a YAML mapping", path))?;

    let values: Vec<String> = root
        .get(list_name)
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if values.iter().any(|value| value == new_value) {
        return Ok(());
    }

    match root.get_mut(list_name).and_then(|v| v.as_sequence_mut()) {
        Some(list) => list.push(Value::String(new_value.to_owned())),
        None => {
            root.insert(
                Value::String(list_name.to_owned()),
                Value::Sequence(vec![Value::String(new_value.to_owned())]),
            );
        }
    }

    file::write_file_content(path, &serde_yaml::to_string(&yaml)?)?;
    Ok(())
}

pub fn remove_from_yaml_list(path: &str, list_name: &str, value: &str) -> Result<()> {
    let content = file::get_file_content(path)?;
    let mut yaml: Value = serde_yaml::from_str(&content)?;
    if let Some(list) = yaml.get_mut(list_name).and_then(|v| v.as_sequence_mut()) {
        list.retain(|item| item.as_str() != Some(value));
    }
    file::write_file_content(path, &serde_yaml::to_string(&yaml)?)?;
    Ok(())
}
