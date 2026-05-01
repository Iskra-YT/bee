use anyhow::Result;

pub fn parse_yaml_file(value: &String) -> Result<serde_yaml::Value> {
    Ok(serde_yaml::from_str(value)?)
}
