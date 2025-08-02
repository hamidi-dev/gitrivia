use anyhow::{bail, Context, Result};
use git2::{BlameOptions, Repository, Sort};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::process::Command;

use rayon::prelude::*;

const MIN_LINES: usize = 25; // ignore trivial files
const ALLOWED_EXT: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "java", "kt", "kts", "go", "py", "rb", "swift", "c", "h",
    "cpp", "hpp", "cc", "hh", "cs", "php", "scala", "m", "mm", "sh", "bash", "zsh", "fish", "sql",
    "xml", "yml", "yaml", "toml", "json", "lua", "vim", "conf", "ini", "cfg", "md",
];

#[derive(Debug, Clone)]
pub struct BusScore {
    pub file: String,
    pub top_author: String,
    pub ratio: f64,   // 0..1
    pub total: usize, // lines (blame) or touches (fast)
}

/// Collect repo-tracked files, filtered by extension.
fn list_repo_files(repo_path: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("ls-files")
        .output()
        .context("failed to run `git ls-files`")?;

    let files = String::from_utf8_lossy(&output.stdout);
    let v = files
        .lines()
        .filter(|file| {
            Path::new(file)
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ALLOWED_EXT.contains(&ext))
                .unwrap_or(false)
        })
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    Ok(v)
}

/// Parallel (multi-core) blame across files.
/// Re-opens the repo per file (cheap vs. blame cost; avoids Send/Sync questions).
pub fn compute_scores_parallel(repo_path: &str) -> Result<Vec<BusScore>> {
    let files = list_repo_files(repo_path)?;
    let scores: Vec<_> = files
        .par_iter()
        .filter_map(|file| {
            let repo = Repository::discover(repo_path).ok()?;
            let mut opts = BlameOptions::new();
            // opts.track_copies_same_file(true).track_copies_any_commit_copies(true); // slower
            let blame = repo.blame_file(Path::new(file), Some(&mut opts)).ok()?;

            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for h in blame.iter() {
                let email = h.final_signature().email().unwrap_or("unknown").to_string();
                *counts.entry(email).or_default() += h.lines_in_hunk() as usize;
            }
            let total: usize = counts.values().copied().sum();
            if total < MIN_LINES {
                return None;
            }
            let (top_author, top_lines) = counts.into_iter().max_by_key(|(_, c)| *c)?;
            let ratio = top_lines as f64 / total as f64;
            Some(BusScore {
                file: file.to_string(),
                top_author,
                ratio,
                total,
            })
        })
        .collect();

    // Sort by ratio desc, then by total desc
    let mut scores = scores;
    scores.sort_by(|a, b| {
        b.ratio
            .partial_cmp(&a.ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    Ok(scores)
}

/// SUPER FAST heuristic: ownership by "touch counts" per author per file
/// over the last `max_commits` commits (or full history if None).
pub fn compute_scores_fast(repo: &Repository, max_commits: Option<usize>) -> Result<Vec<BusScore>> {
    use git2::DiffOptions;

    // file -> author -> touches
    let mut touches: HashMap<String, HashMap<String, usize>> = HashMap::new();

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TIME)?;

    let mut seen = 0usize;
    for oid in walk.flatten() {
        if let Some(m) = max_commits {
            if seen >= m {
                break;
            }
        }
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let email = commit.author().email().unwrap_or("unknown").to_string();

        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Ok(parent) = commit.parent(0) {
            let parent_tree = match parent.tree() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut opt = DiffOptions::new();
            // we only need paths; no per-line stats
            if let Ok(diff) =
                repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opt))
            {
                for d in diff.deltas() {
                    if let Some(path) = d.new_file().path().or_else(|| d.old_file().path()) {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if !ALLOWED_EXT.contains(&ext) {
                                continue;
                            }
                        } else {
                            continue;
                        }
                        let path_str = match path.to_str() {
                            Some(s) => s.to_string(),
                            None => continue,
                        };
                        *touches
                            .entry(path_str)
                            .or_default()
                            .entry(email.clone())
                            .or_default() += 1;
                    }
                }
            }
        }
        seen += 1;
    }

    let mut scores = Vec::<BusScore>::new();
    for (file, by_author) in touches {
        let total: usize = by_author.values().sum();
        if total == 0 {
            continue;
        }
        if let Some((top_author, top)) = by_author.into_iter().max_by_key(|(_, n)| *n) {
            scores.push(BusScore {
                file,
                top_author,
                ratio: top as f64 / total as f64,
                total,
            });
        }
    }

    scores.sort_by(|a, b| {
        b.ratio
            .partial_cmp(&a.ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    Ok(scores)
}

/// Backward-compatible: only return those above threshold (from parallel blame).
pub fn bus_factor(
    repo_path: &str,
    repo: &Repository,
    threshold: f64,
) -> Result<BTreeMap<String, (String, f64)>> {
    if !(0.0..=1.0).contains(&threshold) {
        bail!("threshold must be in [0.0, 1.0]");
    }
    let scores = compute_scores_parallel(repo_path)?;
    let mut warnings = BTreeMap::new();
    for s in scores.into_iter().filter(|s| s.ratio > threshold) {
        warnings.insert(s.file, (s.top_author, s.ratio));
    }
    Ok(warnings)
}

pub fn as_busfactor_json(map: &BTreeMap<String, (String, f64)>) -> String {
    let as_json: BTreeMap<_, _> = map
        .iter()
        .map(|(f, (a, r))| (f.clone(), json!({ "author": a, "ownership": r })))
        .collect();
    serde_json::to_string_pretty(&as_json).unwrap()
}
