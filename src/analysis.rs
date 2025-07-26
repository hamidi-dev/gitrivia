use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike};
use git2::{BlameOptions, Repository, Sort};
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

use crate::utils::{fmt_date, open_repo_or_die};

pub fn print_stats(path: &str, limit: Option<usize>, sort_desc: bool) {
    let repo = open_repo_or_die(path);
    let limit = limit.unwrap_or(usize::MAX);

    let total_commits = count_all_commits(&repo);
    let commits = collect_commits(&repo, limit, None);

    println!("Total commits in repo:   {}", total_commits);
    println!("Total commits scanned:  {}", commits.total_seen);
    println!("— per‑author —");
    for line in commits.formatted_lines(sort_desc) {
        println!("{line}");
    }
}

pub fn top_authors(path: &str, since: Option<NaiveDate>) {
    let repo = open_repo_or_die(path);
    let since_dt = since.map(|d| {
        Local
            .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
            .unwrap()
    });

    let commits = collect_commits(&repo, usize::MAX, since_dt);
    println!("Authors since {:?}:", since);
    for line in commits.formatted_lines(false) {
        println!("{line}");
    }
}

pub fn author_activity(path: &str, author: &str) {
    let repo = open_repo_or_die(path);
    let commits = collect_commits(&repo, usize::MAX, None);

    match commits.data.get(author) {
        Some(meta) => println!(
            "{:<30} {:>4} commits 🗓  {} → {}",
            author,
            meta.count,
            fmt_date(meta.first),
            fmt_date(meta.last)
        ),
        None => eprintln!("No commits by {author}"),
    }
}

/// Internal -----------------------------------------------------------------

struct AuthorMeta {
    count: usize,
    first: DateTime<Local>,
    last: DateTime<Local>,
}

pub(crate) struct CommitStats {
    total_seen: usize,
    data: HashMap<String, AuthorMeta>,
}

impl CommitStats {
    fn formatted_lines(&self, desc: bool) -> Vec<String> {
        let mut v: Vec<_> = self
            .data
            .iter()
            .map(|(email, m)| {
                (
                    m.count,
                    format!(
                        "{:<30} {:>4} commits   🗓  {} → {}",
                        email,
                        m.count,
                        fmt_date(m.first),
                        fmt_date(m.last)
                    ),
                )
            })
            .collect();

        v.sort_by(|a, b| if desc { b.0.cmp(&a.0) } else { a.0.cmp(&b.0) });

        v.into_iter().map(|(_, s)| s).collect()
    }
}

/// Walk the repo and collect per‑author stats.
fn collect_commits(repo: &Repository, limit: usize, since: Option<DateTime<Local>>) -> CommitStats {
    let mut revwalk = repo.revwalk().expect("revwalk");
    revwalk.push_head().unwrap();
    revwalk.set_sorting(Sort::TIME).unwrap();

    let mut data: HashMap<String, AuthorMeta> = HashMap::new();
    let mut seen = 0;

    for id in revwalk.flatten() {
        if seen >= limit {
            break;
        }

        let commit = match repo.find_commit(id) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let email = commit
            .author()
            .email()
            .unwrap_or("unknown@example.com")
            .to_string();

        let ts = commit.time().seconds();
        let dt = Local.timestamp_opt(ts, 0).single().unwrap();

        if let Some(min_dt) = since {
            if dt < min_dt {
                continue;
            }
        }

        seen += 1;
        let entry = data.entry(email).or_insert(AuthorMeta {
            count: 0,
            first: dt,
            last: dt,
        });
        entry.count += 1;
        if dt < entry.first {
            entry.first = dt;
        }
        if dt > entry.last {
            entry.last = dt;
        }
    }

    CommitStats {
        total_seen: seen,
        data,
    }
}

fn count_all_commits(repo: &Repository) -> usize {
    let mut revwalk = repo.revwalk().expect("revwalk");
    revwalk.push_head().unwrap();
    revwalk.set_sorting(Sort::TIME).unwrap();
    revwalk.count()
}

pub fn blame_summary(repo_path: &str, file: &str, json: bool) {
    let repo = open_repo_or_die(repo_path);
    let file_path = Path::new(file);

    let mut opts = BlameOptions::new();
    let blame = repo
        .blame_file(file_path, Some(&mut opts))
        .expect("Could not blame file");

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for hunk in blame.iter() {
        let email = hunk
            .final_signature()
            .email()
            .unwrap_or("unknown")
            .to_string();
        *counts.entry(email).or_default() += hunk.lines_in_hunk() as usize;
    }

    if json {
        let out =
            serde_json::to_string_pretty(&BTreeMap::from([(file.to_string(), counts)])).unwrap();
        println!("{out}");
    } else {
        for (email, count) in counts {
            println!("{:<30} {:>4} lines", email, count);
        }
    }
}

pub fn file_contributions(path: &str, json: bool) {
    let repo = open_repo_or_die(path);
    let mut file_authors: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    for oid in revwalk.flatten() {
        let commit = repo.find_commit(oid).unwrap();
        let tree = commit.tree().unwrap();

        if let Some(parent) = commit.parent(0).ok() {
            let parent_tree = parent.tree().unwrap();
            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)
                .unwrap();
            diff.deltas().for_each(|delta| {
                if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                    let email = commit.author().email().unwrap_or("unknown").to_string();
                    *file_authors
                        .entry(path.to_string())
                        .or_default()
                        .entry(email)
                        .or_default() += 1;
                }
            });
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&file_authors).unwrap());
    } else {
        for (file, authors) in file_authors {
            println!("{file}");
            for (author, count) in authors {
                println!("  {:<30} {} commits", author, count);
            }
        }
    }
}

pub fn commit_times(path: &str, json: bool) {
    let repo = open_repo_or_die(path);
    let mut author_times: BTreeMap<String, BTreeMap<&str, usize>> = BTreeMap::new();

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    for oid in revwalk.flatten() {
        let commit = repo.find_commit(oid).unwrap();
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
        };
        let author = commit.author();
        let email = author.email().unwrap_or("unknown").to_string();
        *author_times
            .entry(email)
            .or_default()
            .entry(bucket)
            .or_default() += 1;
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&author_times).unwrap());
    } else {
        for (email, buckets) in author_times {
            println!("{email}");
            for (label, count) in buckets {
                println!("  {:<10} {}", label, count);
            }
        }
    }
}

pub fn first_commits(path: &str, json: bool) {
    let repo = open_repo_or_die(path);
    let mut firsts: BTreeMap<String, DateTime<Local>> = BTreeMap::new();

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    for oid in revwalk.flatten() {
        let commit = repo.find_commit(oid).unwrap();
        let dt = Local
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .unwrap();
        let email = commit.author().email().unwrap_or("unknown").to_string();

        firsts
            .entry(email.clone())
            .and_modify(|d| {
                if dt < *d {
                    *d = dt
                }
            })
            .or_insert(dt);
    }

    if json {
        let map: BTreeMap<_, _> = firsts.into_iter().map(|(k, v)| (k, fmt_date(v))).collect();
        println!("{}", serde_json::to_string_pretty(&map).unwrap());
    } else {
        for (email, dt) in firsts {
            println!("{:<30} {}", email, fmt_date(dt));
        }
    }
}

pub fn top_coauthors(path: &str, json: bool) {
    let repo = open_repo_or_die(path);
    let mut file_authors: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    for oid in revwalk.flatten() {
        let commit = repo.find_commit(oid).unwrap();
        let tree = commit.tree().unwrap();

        if let Some(parent) = commit.parent(0).ok() {
            let parent_tree = parent.tree().unwrap();
            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)
                .unwrap();
            for delta in diff.deltas() {
                if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                    let author = commit.author().email().unwrap_or("unknown").to_string();
                    let authors = file_authors.entry(path.to_string()).or_default();
                    if !authors.contains(&author) {
                        authors.push(author);
                    }
                }
            }
        }
    }

    let mut pairs: BTreeMap<String, usize> = BTreeMap::new();
    for authors in file_authors.values() {
        for i in 0..authors.len() {
            for j in i + 1..authors.len() {
                let mut pair = vec![authors[i].clone(), authors[j].clone()];
                pair.sort();
                let key = format!("{} + {}", pair[0], pair[1]);
                *pairs.entry(key).or_default() += 1;
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&pairs).unwrap());
    } else {
        for (pair, count) in pairs {
            println!("{:<50} {:>3} shared files", pair, count);
        }
    }
}

use std::process::Command;

pub fn bus_factor(path: &str, json: bool, threshold: f64) {
    let repo = open_repo_or_die(path);
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("ls-files")
        .output()
        .expect("failed to run git ls-files");

    let files = String::from_utf8_lossy(&output.stdout);
    let mut warnings = BTreeMap::new();

    for file in files.lines() {
        if !(file.ends_with(".rs") || file.ends_with(".ts") || file.ends_with(".java")) {
            continue;
        }

        let mut opts = BlameOptions::new();
        let blame = repo.blame_file(Path::new(file), Some(&mut opts));
        if let Ok(blame) = blame {
            let mut counts = BTreeMap::new();
            for h in blame.iter() {
                let sig = h.final_signature();
                let email = sig.email().unwrap_or("unknown").to_string();
                *counts.entry(email).or_insert(0) += h.lines_in_hunk();
            }
            let total: usize = counts.values().copied().sum();
            if total == 0 {
                continue;
            }
            let (top_author, top_lines) = counts.into_iter().max_by_key(|(_, c)| *c).unwrap();
            let ratio = top_lines as f64 / total as f64;
            if ratio > threshold {
                warnings.insert(file.to_string(), (top_author, ratio));
            }
        }
    }

    if json {
        let as_json: BTreeMap<_, _> = warnings
            .into_iter()
            .map(|(f, (a, r))| (f, json!({ "author": a, "ownership": r })))
            .collect();
        println!("{}", serde_json::to_string_pretty(&as_json).unwrap());
    } else {
        for (file, (author, ratio)) in warnings {
            println!("⚠️  {:<30} {:>5.1}% by {}", file, ratio * 100.0, author);
        }
    }
}
