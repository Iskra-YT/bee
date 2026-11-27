use anyhow::{anyhow, Result};
use chrono::Utc;
use std::time::Duration;

pub fn get_timestamp_string() -> String {
    let now = Utc::now();
    now.to_rfc3339()
}

/// Parse a task timeout such as `30`, `30s`, `5m`, `1h` or `500ms`.
/// A bare number means seconds. Zero is rejected.
pub fn parse_duration(value: &str) -> Result<Duration> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("timeout cannot be empty"));
    }

    let (amount, multiplier) = if let Some(raw) = value.strip_suffix("ms") {
        (raw, 1)
    } else if let Some(raw) = value.strip_suffix('s') {
        (raw, 1_000)
    } else if let Some(raw) = value.strip_suffix('m') {
        (raw, 60_000)
    } else if let Some(raw) = value.strip_suffix('h') {
        (raw, 3_600_000)
    } else {
        (value, 1_000)
    };

    let amount: u64 = amount
        .parse()
        .map_err(|_| anyhow!("invalid timeout '{}': expected a number with optional s/m/h/ms suffix", value))?;
    if amount == 0 {
        return Err(anyhow!("invalid timeout '{}': must be greater than zero", value));
    }
    let millis = amount.checked_mul(multiplier)
        .ok_or_else(|| anyhow!("invalid timeout '{}': value is too large", value))?;
    Ok(Duration::from_millis(millis))
}
