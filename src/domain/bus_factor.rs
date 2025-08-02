use anyhow::{bail, Context, Result};
use git2::{BlameOptions, Repository};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// Returns map: file -> (top_author, ownership_ratio)
pub fn bus_factor(repo_path: &str, repo: &Repository, threshold: f64)
    -> Result<BTreeMap<String, (String, f64)>>
{
    if !(0.0..=1.0).contains(&threshold) {
        bail!("threshold must be in [0.0, 1.0]");
    }

    let output = Command::new("git")
        .arg("-C").arg(repo_path)
        .arg("ls-files")
        .output()
        .context("failed to run `git ls-files`")?;

    let files = String::from_utf8_lossy(&output.stdout);
    let mut warnings = BTreeMap::new();

    for file in files.lines() {
        if !(file.ends_with(".rs") || file.ends_with(".ts") || file.ends_with(".java")) {
            continue;
        }

        let mut opts = BlameOptions::new();
        if let Ok(blame) = repo.blame_file(Path::new(file), Some(&mut opts)) {
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for h in blame.iter() {
                let email = h.final_signature().email().unwrap_or("unknown").to_string();
                *counts.entry(email).or_default() += h.lines_in_hunk() as usize;
            }
            let total: usize = counts.values().copied().sum();
            if total == 0 { continue; }
            let (top_author, top_lines) = counts.into_iter().max_by_key(|(_,c)| *c).unwrap();
            let ratio = top_lines as f64 / total as f64;
            if ratio > threshold {
                warnings.insert(file.to_string(), (top_author, ratio));
            }
        }
    }
    Ok(warnings)
}

/// Optional helper if du JSON fertigmachen willst:
pub fn as_busfactor_json(map: &BTreeMap<String, (String, f64)>) -> String {
    let as_json: BTreeMap<_, _> = map.iter()
        .map(|(f,(a,r))| (f.clone(), json!({ "author": a, "ownership": r })))
        .collect();
    serde_json::to_string_pretty(&as_json).unwrap()
}

