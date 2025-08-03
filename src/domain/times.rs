use anyhow::Result;
use chrono::{Local, TimeZone, Timelike};
use git2::Repository;
use std::collections::BTreeMap;

pub fn commit_times(repo: &Repository) -> Result<BTreeMap<String, BTreeMap<String, usize>>> {
    let mut author_times: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut rw = repo.revwalk()?;
    rw.push_head()?;

    for oid in rw.flatten() {
        let commit = repo.find_commit(oid)?;
        let dt = Local
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .unwrap();
        let hour = dt.hour();
        let bucket = match hour {
            0..=5 => "night",
            6..=11 => "morning",
            12..=17 => "afternoon",
            _ => "evening",
        }
        .to_string();

        let email = commit.author().email().unwrap_or("unknown").to_string();
        *author_times
            .entry(email)
            .or_default()
            .entry(bucket)
            .or_default() += 1;
    }
    Ok(author_times)
}
