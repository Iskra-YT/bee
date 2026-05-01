use chrono::Utc;

pub fn get_timestamp_string() -> String {
    let now = Utc::now();
    now.to_rfc3339()
}