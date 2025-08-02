use anyhow::{bail, Context, Result};
use git2::{BlameOptions, DiffOptions, Repository, Sort};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::process::Command;

use rayon::prelude::*;

/// Default minimum size to report (lines in blame mode / touches in fast mode).
const DEFAULT_MIN_TOTAL: usize = 25;

/// Built-in allow-list; can be extended or bypassed via ScanOpts.
const ALLOWED_EXT: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "java", "kt", "kts", "go", "py", "rb", "swift", "c", "h",
    "cpp", "hpp", "cc", "hh", "cs", "php", "scala", "m", "mm", "sh", "bash", "zsh", "fish", "sql",
    "xml", "yml", "yaml", "toml", "json", "lock", "lua", "vim", "conf", "ini", "cfg", "md", "txt",
];

#[derive(Debug, Clone)]
pub struct BusScore {
    pub file: String,
    pub top_author: String,
    /// 0..1 ownership share (lines in blame mode, touches in fast mode)
    pub ratio: f64,
    /// Total lines (blame) or touches (fast) considered for the file
    pub total: usize,
}

#[derive(Debug, Clone)]
pub struct ScanOpts {
    /// Include ALL tracked files (ignore extension allow-list)
    pub all: bool,
    /// Extra extensions to include (e.g. ["lua", "vim"])
    pub include_ext: Vec<String>,
    /// Minimum total (lines/touches) to keep a file in results
    pub min_total: usize,
}

impl Default for ScanOpts {
    fn default() -> Self {
        Self {
            all: false,
            include_ext: Vec::new(),
            min_total: DEFAULT_MIN_TOTAL,
        }
    }
}

fn ext_ok(file: &str, opts: &ScanOpts) -> bool {
    if opts.all {
        return true;
    }
    let ext = Path::new(file).extension().and_then(|e| e.to_str());
    match ext {
        Some(e) => {
            let e = e.to_ascii_lowercase();
            ALLOWED_EXT.contains(&e.as_str()) || opts.include_ext.iter().any(|x| x == &e)
        }
        None => false,
    }
}

/// List all tracked files (no filtering; filtering is done later).
fn list_repo_files(repo_path: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("ls-files")
        .output()
        .context("failed to run `git ls-files`")?;

    let files = String::from_utf8_lossy(&output.stdout);
    Ok(files.lines().map(|s| s.to_string()).collect())
}

/// Parallel blame across files (accurate, uses all cores).
pub fn compute_scores_parallel(repo_path: &str, opts: &ScanOpts) -> Result<Vec<BusScore>> {
    let files = list_repo_files(repo_path)?;
    let scores: Vec<_> = files
        .par_iter()
        .filter_map(|file| {
            if !ext_ok(file, opts) {
                return None;
            }
            // open a repo handle per task (cheap vs blame cost; avoids Send/Sync issues)
            let repo = Repository::discover(repo_path).ok()?;
            let mut blame_opts = BlameOptions::new();
            // You may enable copy tracking, but it can be significantly slower:
            // blame_opts.track_copies_same_file(true).track_copies_any_commit_copies(true);

            let blame = repo
                .blame_file(Path::new(file), Some(&mut blame_opts))
                .ok()?;

            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for h in blame.iter() {
                let email = h.final_signature().email().unwrap_or("unknown").to_string();
                *counts.entry(email).or_default() += h.lines_in_hunk() as usize;
            }
            let total: usize = counts.values().copied().sum();
            if total < opts.min_total {
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

    let mut scores = scores;
    // Sort by ownership ratio desc, then by size desc
    scores.sort_by(|a, b| {
        b.ratio
            .partial_cmp(&a.ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    Ok(scores)
}

/// SUPER FAST heuristic: ownership by "touch counts" per author per file.
/// No blame; looks at diffs across history (or last N commits).
pub fn compute_scores_fast(
    repo: &Repository,
    max_commits: Option<usize>,
    opts: &ScanOpts,
) -> Result<Vec<BusScore>> {
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
            // We only need paths (not per-line stats)
            if let Ok(diff) =
                repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opt))
            {
                for d in diff.deltas() {
                    if let Some(path) = d.new_file().path().or_else(|| d.old_file().path()) {
                        if let Some(path_str) = path.to_str() {
                            if !ext_ok(path_str, opts) {
                                continue;
                            }
                            *touches
                                .entry(path_str.to_string())
                                .or_default()
                                .entry(email.clone())
                                .or_default() += 1;
                        }
                    }
                }
            }
        }
        seen += 1;
    }

    let mut scores = Vec::<BusScore>::new();
    for (file, by_author) in touches {
        let total: usize = by_author.values().sum();
        if total < opts.min_total {
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

    // Sort by ratio desc, then by size desc
    scores.sort_by(|a, b| {
        b.ratio
            .partial_cmp(&a.ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    Ok(scores)
}

/// Backward-compatible map of warnings: only files above `threshold`.
pub fn bus_factor(
    repo_path: &str,
    _repo: &Repository,
    threshold: f64,
    opts: &ScanOpts,
) -> Result<BTreeMap<String, (String, f64)>> {
    if !(0.0..=1.0).contains(&threshold) {
        bail!("threshold must be in [0.0, 1.0]");
    }
    let scores = compute_scores_parallel(repo_path, opts)?;
    let mut warnings = BTreeMap::new();
    for s in scores.into_iter().filter(|s| s.ratio > threshold) {
        warnings.insert(s.file, (s.top_author, s.ratio));
    }
    Ok(warnings)
}

/// JSON helper for the classic warnings map.
pub fn as_busfactor_json(map: &BTreeMap<String, (String, f64)>) -> String {
    let as_json: BTreeMap<_, _> = map
        .iter()
        .map(|(f, (a, r))| (f.clone(), json!({ "author": a, "ownership": r })))
        .collect();
    serde_json::to_string_pretty(&as_json).unwrap()
}
