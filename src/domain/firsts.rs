use anyhow::Result;
use chrono::{DateTime, Local, TimeZone};
use git2::Repository;
use std::collections::BTreeMap;

pub fn first_commits(repo: &Repository) -> Result<BTreeMap<String, DateTime<Local>>> {
    let mut firsts: BTreeMap<String, DateTime<Local>> = BTreeMap::new();
    let mut rw = repo.revwalk()?;
    rw.push_head()?;

    for oid in rw.flatten() {
        let commit = repo.find_commit(oid)?;
        let dt = Local.timestamp_opt(commit.time().seconds(), 0).single().unwrap();
        let email = commit.author().email().unwrap_or("unknown").to_string();

        firsts.entry(email.clone())
            .and_modify(|d| if dt < *d { *d = dt })
            .or_insert(dt);
    }
    Ok(firsts)
}

