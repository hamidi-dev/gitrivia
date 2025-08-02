use anyhow::{bail, Context, Result};
use git2::{BlameOptions, DiffOptions, Repository, Sort};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path};
use std::process::Command;

use rayon::prelude::*;

pub const ALLOWED_EXT: &[&str] = &[
    "rs","ts","tsx","js","jsx","java","kt","kts","go","py","rb","swift",
    "c","h","cpp","hpp","cc","hh","cs","php","scala","m","mm",
    "sh","bash","zsh","fish","sql","xml","yml","yaml","toml","json","lock",
    "lua","vim","conf","ini","cfg","md","txt",
];

/// Default minimum size to report (lines in blame mode / touches in fast mode).
const DEFAULT_MIN_TOTAL: usize = 25;

#[derive(Debug, Clone)]
pub struct BusScore {
    pub file: String,
    pub top_author: String,
    pub ratio: f64,   // 0..1
    pub total: usize, // lines (blame) or touches (fast)
}

/// Directory-level score
#[derive(Debug, Clone)]
pub struct DirScore {
    pub dir: String,
    pub top_author: String,
    pub ratio: f64,
    pub total: usize, // sum of lines/touches for the directory
}

#[derive(Debug, Clone)]
pub struct ScanOpts {
    pub all: bool,
    pub include_ext: Vec<String>,
    pub min_total: usize, // lines (blame) or touches (fast)
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
    if opts.all { return true; }
    let ext = Path::new(file).extension().and_then(|e| e.to_str());
    match ext {
        Some(e) => {
            let e = e.to_ascii_lowercase();
            ALLOWED_EXT.contains(&e.as_str()) || opts.include_ext.iter().any(|x| x == &e)
        }
        None => false,
    }
}

/// Directory key of depth ("a/b") for a file path.
fn dir_key(path_str: &str, depth: usize) -> String {
    let p = Path::new(path_str);
    let mut parts = Vec::new();
    for c in p.components() {
        if let Component::Normal(os) = c {
            parts.push(os.to_string_lossy().to_string());
        }
    }
    if parts.is_empty() { return ".".into(); }
    parts.pop(); // drop file
    if parts.is_empty() { return ".".into(); }
    let d = parts.len().min(depth.max(1));
    parts[..d].join("/")
}

/// List tracked files
fn list_repo_files(repo_path: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C").arg(repo_path)
        .arg("ls-files")
        .output()
        .context("failed to run `git ls-files`")?;
    let files = String::from_utf8_lossy(&output.stdout);
    Ok(files.lines().map(|s| s.to_string()).collect())
}

/// Parallel blame across files (accurate).
pub fn compute_scores_parallel(repo_path: &str, opts: &ScanOpts) -> Result<Vec<BusScore>> {
    let files = list_repo_files(repo_path)?;
    let scores: Vec<_> = files
        .par_iter()
        .filter_map(|file| {
            if !ext_ok(file, opts) { return None; }
            let repo = Repository::discover(repo_path).ok()?;
            let mut blame_opts = BlameOptions::new();
            let blame = repo.blame_file(Path::new(file), Some(&mut blame_opts)).ok()?;

            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for h in blame.iter() {
                let email = h.final_signature().email().unwrap_or("unknown").to_string();
                *counts.entry(email).or_default() += h.lines_in_hunk() as usize;
            }
            let total: usize = counts.values().copied().sum();
            if total < opts.min_total { return None; }
            let (top_author, top_lines) = counts.into_iter().max_by_key(|(_, c)| *c)?;
            let ratio = top_lines as f64 / total as f64;

            Some(BusScore { file: file.to_string(), top_author, ratio, total })
        })
        .collect();

    let mut scores = scores;
    scores.sort_by(|a, b| {
        b.ratio
            .partial_cmp(&a.ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    Ok(scores)
}

/// SUPER FAST heuristic: ownership by "touch counts" per author per file.
pub fn compute_scores_fast(repo: &Repository, max_commits: Option<usize>, opts: &ScanOpts) -> Result<Vec<BusScore>> {
    // file -> author -> touches
    let mut touches: HashMap<String, HashMap<String, usize>> = HashMap::new();

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TIME)?;

    let mut seen = 0usize;
    for oid in walk.flatten() {
        if let Some(m) = max_commits { if seen >= m { break; } }
        let commit = match repo.find_commit(oid) { Ok(c) => c, Err(_) => continue };
        let email = commit.author().email().unwrap_or("unknown").to_string();

        let tree = match commit.tree() { Ok(t) => t, Err(_) => continue };
        if let Ok(parent) = commit.parent(0) {
            let parent_tree = match parent.tree() { Ok(t) => t, Err(_) => continue };
            let mut opt = DiffOptions::new();
            if let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opt)) {
                for d in diff.deltas() {
                    if let Some(path) = d.new_file().path().or_else(|| d.old_file().path()) {
                        if let Some(p) = path.to_str() {
                            if !ext_ok(p, opts) { continue; }
                            *touches.entry(p.to_string()).or_default().entry(email.clone()).or_default() += 1;
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
        if total < opts.min_total { continue; }
        if let Some((top_author, top)) = by_author.into_iter().max_by_key(|(_, n)| *n) {
            scores.push(BusScore { file, top_author, ratio: top as f64 / total as f64, total });
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

/// Backward-compatible map of warnings: only files above `threshold` (blame mode).
pub fn bus_factor(repo_path: &str, _repo: &Repository, threshold: f64, opts: &ScanOpts)
    -> Result<BTreeMap<String, (String, f64)>>
{
    if !(0.0..=1.0).contains(&threshold) { bail!("threshold must be in [0.0, 1.0]"); }
    let scores = compute_scores_parallel(repo_path, opts)?;
    let mut warnings = BTreeMap::new();
    for s in scores.into_iter().filter(|s| s.ratio > threshold) {
        warnings.insert(s.file, (s.top_author, s.ratio));
    }
    Ok(warnings)
}

pub fn as_busfactor_json(map: &BTreeMap<String, (String, f64)>) -> String {
    let as_json: BTreeMap<_, _> =
        map.iter().map(|(f,(a,r))| (f.clone(), json!({ "author": a, "ownership": r }))).collect();
    serde_json::to_string_pretty(&as_json).unwrap()
}

// ---------------------- NEW: directory-level aggregation -------------------

/// Aggregate file scores into directories (approx via top-owner per file).
/// NOTE: For precise dir aggregation in blame mode (owner shares), use `compute_dir_scores_parallel` instead.
pub fn aggregate_dir_from_file_scores(scores: &[BusScore], depth: usize) -> Vec<DirScore> {
    let mut by_dir: HashMap<String, HashMap<String, usize>> = HashMap::new(); // dir -> author -> total
    let mut totals: HashMap<String, usize> = HashMap::new();

    for s in scores {
        let key = dir_key(&s.file, depth);
        // Approximate: attribute the whole file's total to its top author
        let a = by_dir.entry(key.clone()).or_default();
        *a.entry(s.top_author.clone()).or_default() += s.total;
        *totals.entry(key).or_default() += s.total;
    }

    let mut out = Vec::<DirScore>::new();
    for (dir, authors) in by_dir {
        let total = *totals.get(&dir).unwrap_or(&0);
        if total == 0 { continue; }
        let (top_author, top) = authors.into_iter().max_by_key(|(_, n)| *n).unwrap();
        out.push(DirScore { dir, top_author, ratio: top as f64 / total as f64, total });
    }
    out.sort_by(|a,b| {
        b.ratio.partial_cmp(&a.ratio).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    out
}

/// Accurate dir scores via blame (sum per-author line counts across files in the directory).
pub fn compute_dir_scores_parallel(repo_path: &str, opts: &ScanOpts, depth: usize) -> Result<Vec<DirScore>> {
    let files = list_repo_files(repo_path)?;
    // Produce per-file author->lines maps in parallel
    let per_file: Vec<_> = files.par_iter()
        .filter_map(|file| {
            if !ext_ok(file, opts) { return None; }
            let repo = Repository::discover(repo_path).ok()?;
            let mut blame_opts = BlameOptions::new();
            let blame = repo.blame_file(Path::new(file), Some(&mut blame_opts)).ok()?;
            let mut counts: HashMap<String, usize> = HashMap::new();
            for h in blame.iter() {
                let email = h.final_signature().email().unwrap_or("unknown").to_string();
                *counts.entry(email).or_default() += h.lines_in_hunk() as usize;
            }
            let total: usize = counts.values().copied().sum();
            if total < opts.min_total { return None; }
            Some((file.to_string(), counts, total))
        })
        .collect();

    // Aggregate per directory
    let mut dir_author: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut dir_total: HashMap<String, usize> = HashMap::new();
    for (file, counts, total) in per_file {
        let key = dir_key(&file, depth);
        *dir_total.entry(key.clone()).or_default() += total;
        let da = dir_author.entry(key).or_default();
        for (a, n) in counts {
            *da.entry(a).or_default() += n;
        }
    }

    let mut out = Vec::<DirScore>::new();
    for (dir, authors) in dir_author {
        let total = *dir_total.get(&dir).unwrap_or(&0);
        if total == 0 { continue; }
        let (top_author, top) = authors.into_iter().max_by_key(|(_, n)| *n).unwrap();
        out.push(DirScore { dir, top_author, ratio: top as f64 / total as f64, total });
    }
    out.sort_by(|a,b| {
        b.ratio.partial_cmp(&a.ratio).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    Ok(out)
}

/// Directory scores in FAST mode (touch counts aggregated).
pub fn compute_dir_scores_fast(repo: &Repository, max_commits: Option<usize>, opts: &ScanOpts, depth: usize) -> Result<Vec<DirScore>> {
    // file -> author -> touches
    let mut touches: HashMap<String, HashMap<String, usize>> = HashMap::new();

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TIME)?;

    let mut seen = 0usize;
    for oid in walk.flatten() {
        if let Some(m) = max_commits { if seen >= m { break; } }
        let commit = match repo.find_commit(oid) { Ok(c) => c, Err(_) => continue };
        let email = commit.author().email().unwrap_or("unknown").to_string();

        let tree = match commit.tree() { Ok(t) => t, Err(_) => continue };
        if let Ok(parent) = commit.parent(0) {
            let parent_tree = match parent.tree() { Ok(t) => t, Err(_) => continue };
            let mut opt = DiffOptions::new();
            if let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opt)) {
                for d in diff.deltas() {
                    if let Some(path) = d.new_file().path().or_else(|| d.old_file().path()) {
                        if let Some(p) = path.to_str() {
                            if !ext_ok(p, opts) { continue; }
                            *touches.entry(p.to_string()).or_default().entry(email.clone()).or_default() += 1;
                        }
                    }
                }
            }
        }
        seen += 1;
    }

    // fold into directories
    let mut dir_author: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut dir_total: HashMap<String, usize> = HashMap::new();
    for (file, by_auth) in touches {
        let total: usize = by_auth.values().sum();
        if total < opts.min_total { continue; }
        let key = dir_key(&file, depth);
        *dir_total.entry(key.clone()).or_default() += total;
        let da = dir_author.entry(key).or_default();
        for (a, n) in by_auth {
            *da.entry(a).or_default() += n;
        }
    }

    let mut out = Vec::<DirScore>::new();
    for (dir, authors) in dir_author {
        let total = *dir_total.get(&dir).unwrap_or(&0);
        if total == 0 { continue; }
        let (top_author, top) = authors.into_iter().max_by_key(|(_, n)| *n).unwrap();
        out.push(DirScore { dir, top_author, ratio: top as f64 / total as f64, total });
    }
    out.sort_by(|a,b| {
        b.ratio.partial_cmp(&a.ratio).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    Ok(out)
}

