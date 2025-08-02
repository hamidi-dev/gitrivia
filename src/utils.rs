use chrono::{DateTime, Local};

pub fn fmt_date(dt: DateTime<Local>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

