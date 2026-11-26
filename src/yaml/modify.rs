use crate::file;
use anyhow::Result;
use serde_yaml::Value;

pub fn append_to_yaml_list(path: &str, list_name: &str, new_value: &str) -> Result<()> {
    let content = file::get_file_content(path)?;
    let mut yaml: Value = serde_yaml::from_str(&content)?;

    if let Some(list) = yaml.get_mut(list_name).and_then(|v| v.as_sequence_mut()) {
        list.push(Value::String(new_value.to_owned()));
    } else {
        yaml.as_mapping_mut().unwrap().insert(
            Value::String(list_name.to_owned()),
            Value::Sequence(vec![Value::String(new_value.to_owned())]),
        );
    }

    let new_content = serde_yaml::to_string(&yaml)?;
    file::write_file_content(path, &new_content)?;

    Ok(())
}

pub fn remove_from_yaml_list(path: &str, list_name: &str, value: &str) -> Result<()> {
    let content = file::get_file_content(path)?;
    let mut yaml: Value = serde_yaml::from_str(&content)?;

    if let Some(list) = yaml.get_mut(list_name).and_then(|v| v.as_sequence_mut()) {
        list.retain(|item| item.as_str() != Some(value));
    }

    let new_content = serde_yaml::to_string(&yaml)?;
    file::write_file_content(path, &new_content)?;

    Ok(())
}
