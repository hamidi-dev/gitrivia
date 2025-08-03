use anyhow::Result;
use git2::{BlameOptions, Repository};
use std::collections::BTreeMap;
use std::path::Path;

pub fn blame_counts(repo: &Repository, file: &Path) -> Result<BTreeMap<String, usize>> {
    let mut opts = BlameOptions::new();
    let blame = repo.blame_file(file, Some(&mut opts))?;

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for h in blame.iter() {
        let email = h.final_signature().email().unwrap_or("unknown").to_string();
        *counts.entry(email).or_default() += h.lines_in_hunk() as usize;
    }
    Ok(counts)
}
