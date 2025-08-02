use anyhow::Result;
use chrono::{Duration, Local, TimeZone};
use git2::{DiffOptions, Patch, Repository, Sort};
use std::collections::HashMap;
use std::path::{Component, Path};

use crate::domain::bus_factor::ScanOpts;

/// Per-file churn stats (windowed).
#[derive(Debug, Clone)]
pub struct ChurnEntry {
    pub path: String,
    pub churn: f64,   // weighted sum of (adds + dels)
    pub adds: usize,
    pub dels: usize,
    pub touches: usize,
}

/// Compute churn over recent history (window_days). Churn per file is
/// sum over commits in window of: (adds + dels) * linear_decay(age_days).
/// - Filtering by extension via ScanOpts.
/// - Uses per-delta Patch line stats to count adds/dels.
pub fn compute_churn(repo: &Repository, window_days: i64, opts: &ScanOpts) -> Result<Vec<ChurnEntry>> {
    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TIME)?;

    // Use "now" as upper bound. (Alternative: use repo's last commit timestamp.)
    let now = Local::now();
    let threshold = now - Duration::days(window_days.max(0));

    // path -> (churn f64, adds, dels, touches)
    let mut by_file: HashMap<String, (f64, usize, usize, usize)> = HashMap::new();

    for oid in walk.flatten() {
        let commit = match repo.find_commit(oid) { Ok(c) => c, Err(_) => continue };
        let dt = Local.timestamp_opt(commit.time().seconds(), 0).single().unwrap_or(now);
        if dt < threshold { continue; }

        let tree = match commit.tree() { Ok(t) => t, Err(_) => continue };
        if let Ok(parent) = commit.parent(0) {
            let parent_tree = match parent.tree() { Ok(t) => t, Err(_) => continue };
            let mut opt = DiffOptions::new();
            // Do not enable rename detection here for speed; we only need line counts.
            let diff = match repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opt)) {
                Ok(d) => d, Err(_) => continue
            };

            // Linear decay inside the window: 1.0 for newest, ~0.0 for oldest
            let age_days = (now - dt).num_days().max(0) as f64;
            let w = if window_days > 0 { ((window_days as f64) - age_days).max(0.0) / (window_days as f64) } else { 1.0 };

            for (i, d) in diff.deltas().enumerate() {
                // Prefer new path, fallback to old path
                let path_opt = d.new_file().path().or_else(|| d.old_file().path());
                let path_str = match path_opt.and_then(|p| p.to_str()) { Some(s) => s, None => continue };

                if !ext_ok(path_str, opts) { continue; }

                // Per-delta patch for line stats
                if let Ok(Some(patch)) = Patch::from_diff(&diff, i) {
                    // Signature: (context, additions, deletions)
                    let (ctx, adds, dels) = patch.line_stats().unwrap_or((0, 0, 0));
                    let change = adds + dels;
                    if change == 0 && ctx == 0 { continue; }

                    let entry = by_file.entry(path_str.to_string()).or_insert((0.0, 0, 0, 0));
                    entry.0 += (change as f64) * w;
                    entry.1 += adds;
                    entry.2 += dels;
                    entry.3 += 1; // touches
                }
            }
        }
    }

    let mut out: Vec<ChurnEntry> = by_file.into_iter().map(|(path, (churn, adds, dels, touches))| {
        ChurnEntry { path, churn, adds, dels, touches }
    }).collect();

    out.sort_by(|a, b| b.churn.partial_cmp(&a.churn).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

// --- local helpers (keep consistent with bus_factor) ---

fn ext_ok(file: &str, opts: &ScanOpts) -> bool {
    if opts.all { return true; }
    let ext = Path::new(file).extension().and_then(|e| e.to_str());
    match ext {
        Some(e) => {
            let e = e.to_ascii_lowercase();
            super::bus_factor::ALLOWED_EXT.contains(&e.as_str()) || opts.include_ext.iter().any(|x| x == &e)
        }
        None => false,
    }
}

/// Build a directory key of given depth ("a/b") for a file path.
/// Depth counts from the repo root; if depth <= 0 or no parent directories,
/// returns "." (root).
pub fn dir_key(path_str: &str, depth: usize) -> String {
    let p = Path::new(path_str);
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(os) => parts.push(os.to_string_lossy().to_string()),
            _ => {}
        }
    }
    if parts.is_empty() { return ".".into(); }
    // drop filename
    parts.pop();
    if parts.is_empty() { return ".".into(); }
    let d = parts.len().min(depth.max(1));
    parts[..d].join("/")
}

