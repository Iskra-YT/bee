use serde_yaml::Value;
use anyhow::Result;
use crate::file;

pub fn append_to_yaml_list(path: &String, list_name: &String, new_value: &String) -> Result<()> {
    let content = file::get_file_content(path)?;
    let mut yaml: Value = serde_yaml::from_str(&content)?;

    if let Some(list) = yaml.get_mut(list_name).and_then(|v| v.as_sequence_mut()) {
        list.push(Value::String(new_value.clone()));
    } else {
        yaml.as_mapping_mut().unwrap().insert(Value::String(list_name.clone()), Value::Sequence(vec![Value::String(new_value.clone())]));
    }

    let new_content = serde_yaml::to_string(&yaml)?;
    file::write_file_content(path, &new_content)?;

    Ok(())
}