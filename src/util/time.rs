use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

pub fn format_timestamp(ts: u64) -> String {
    if ts == 0 {
        return "—".to_string();
    }
    let days = ts / 86400;
    let rem = ts % 86400;
    let hours = rem / 3600;
    let mins = (rem % 3600) / 60;
    format!("{:02}/{:02} {:02}:{:02}", (days % 365) % 12 + 1, days % 28 + 1, hours, mins)
}

pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
