use chrono::{DateTime, Local};
use git2::Repository;
use std::process;

pub fn open_repo_or_die(path: &str) -> Repository {
    Repository::discover(path).unwrap_or_else(|e| {
        eprintln!("Error opening repo: {e}");
        process::exit(1);
    })
}

pub fn fmt_date(dt: DateTime<Local>) -> String {
    dt.format("%Y‑%m‑%d").to_string()
}

