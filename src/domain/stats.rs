use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Weekday};
use git2::{Repository, Sort};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct AuthorMeta {
    pub count: usize,
    pub first: DateTime<Local>,
    pub last: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct CommitStats {
    pub total_seen: usize,
    pub data: HashMap<String, AuthorMeta>,
}

impl CommitStats {
    pub fn formatted_lines(&self, desc: bool) -> Vec<String> {
        use crate::utils::fmt_date;
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

#[derive(Debug, Clone)]
pub struct RepoSummary {
    pub total_commits: usize,
    pub contributors_total: usize,
    pub first_author: String,
    pub first_date: DateTime<Local>,
    pub last_author: String,
    pub last_date: DateTime<Local>,
    pub active_days: i64,
    pub avg_commits_per_day: f64,
    pub peak_day: Option<(NaiveDate, usize)>,
    pub active_authors_last_90d: usize,
    pub top_recent_30d: Option<(String, usize)>,

    // New insights:
    pub drive_by_ratio: f64,        // authors with ≤2 commits / total authors
    pub core_size_80pct: usize,     // #authors covering ≥80% of commits
    pub hhi: f64,                   // Σ share^2 over authors (0..1)
    pub gini: f64,                  // inequality of contributions (0..1)
    pub longest_idle_gap_days: i64, // max days between two consecutive commits
    pub weekday_counts: [usize; 7], // Mon..Sun counts
    pub work_hours_pct: f64,        // commits between 09:00–17:59 local
    pub merge_rate: f64,            // merge commits / total
    pub revert_rate: f64,           // reverts / total (heuristic)
    pub msg_median_len: usize,      // subject length median (chars)
    pub msg_body_pct: f64,          // % commits with a non-empty body
    pub conv_commit_pct: f64,       // % subjects matching Conventional Commits
    pub momentum_90d_pct: f64,      // commits in last 90d / total (%)
}

#[derive(Debug, Clone)]
pub struct RepoScan {
    pub stats: CommitStats,
    pub summary: RepoSummary,
    pub recent12: CommitStats,
}

/// Count all commits.
pub fn count_all_commits(repo: &Repository) -> usize {
    let mut rw = repo.revwalk().expect("revwalk");
    rw.push_head().unwrap();
    rw.set_sorting(Sort::TIME).unwrap();
    rw.count()
}

/// Old API: per-author stats (kept for other commands)
pub fn collect_commits(
    repo: &Repository,
    limit: usize,
    since: Option<DateTime<Local>>,
) -> CommitStats {
    let mut rw = repo.revwalk().expect("revwalk");
    rw.push_head().unwrap();
    rw.set_sorting(Sort::TIME).unwrap();

    let mut data = HashMap::<String, AuthorMeta>::new();
    let mut seen = 0usize;

    for id in rw.flatten() {
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
        let dt = Local
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .unwrap();

        if let Some(min) = since {
            if dt < min {
                continue;
            }
        }

        seen += 1;
        let e = data.entry(email).or_insert(AuthorMeta {
            count: 0,
            first: dt,
            last: dt,
        });
        e.count += 1;
        if dt < e.first {
            e.first = dt;
        }
        if dt > e.last {
            e.last = dt;
        }
    }
    CommitStats {
        total_seen: seen,
        data,
    }
}

/// New single-pass scanner computing author stats + rich repo summary.
pub fn scan_repo(repo: &Repository, limit: Option<usize>) -> RepoScan {
    let mut rw = repo.revwalk().expect("revwalk");
    rw.push_head().unwrap();
    rw.set_sorting(Sort::TIME).unwrap();

    let mut data = HashMap::<String, AuthorMeta>::new();
    let mut seen = 0usize;

    let mut first_date: Option<DateTime<Local>> = None;
    let mut first_author = String::new();
    let mut last_date: Option<DateTime<Local>> = None;
    let mut last_author = String::new();

    let mut contributors = HashSet::<String>::new();
    let mut day_counts = BTreeMap::<NaiveDate, usize>::new();

    // Extras
    let mut all_dates: Vec<NaiveDate> = Vec::new(); // for idle gap
    let mut weekday_counts = [0usize; 7]; // Mon..Sun
    let mut work_hours_hits = 0usize;

    let mut merges = 0usize;
    let mut reverts = 0usize;

    let mut subj_lens: Vec<usize> = Vec::new();
    let mut body_hits = 0usize;
    let mut conv_hits = 0usize;

    // Recent windows computed after knowing last_date
    let mut commits_log: Vec<(String, DateTime<Local>)> = Vec::new();

    for id in rw.flatten() {
        if let Some(max) = limit {
            if seen >= max {
                break;
            }
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

        // per-author stats
        seen += 1;
        let e = data.entry(email.clone()).or_insert(AuthorMeta {
            count: 0,
            first: dt,
            last: dt,
        });
        e.count += 1;
        if dt < e.first {
            e.first = dt;
        }
        if dt > e.last {
            e.last = dt;
        }

        // summary core
        contributors.insert(email.clone());
        let day = dt.date_naive();
        *day_counts.entry(day).or_default() += 1;

        match first_date {
            None => {
                first_date = Some(dt);
                first_author = email.clone();
            }
            Some(d) => {
                if dt < d {
                    first_date = Some(dt);
                    first_author = email.clone();
                }
            }
        }
        match last_date {
            None => {
                last_date = Some(dt);
                last_author = email.clone();
            }
            Some(d) => {
                if dt > d {
                    last_date = Some(dt);
                    last_author = email.clone();
                }
            }
        }

        // extras
        all_dates.push(day);

        // weekday 0..6 (Mon..Sun)
        let idx = match dt.weekday() {
            Weekday::Mon => 0,
            Weekday::Tue => 1,
            Weekday::Wed => 2,
            Weekday::Thu => 3,
            Weekday::Fri => 4,
            Weekday::Sat => 5,
            Weekday::Sun => 6,
        };
        weekday_counts[idx] += 1;

        let hour = dt.time().hour(); // requires Timelike via prelude
        if (9..=17).contains(&hour) {
            work_hours_hits += 1;
        }

        if commit.parent_count() > 1 {
            merges += 1;
        }
        let subject = commit.summary().unwrap_or("").trim();
        let message = commit.message().unwrap_or("").trim();

        if subject.starts_with("Revert") || message.contains("This reverts commit") {
            reverts += 1;
        }

        // message hygiene
        subj_lens.push(subject.chars().count());
        // body present? crude but effective: look for blank line then non-empty
        let body_present = message
            .splitn(2, "\n\n")
            .nth(1)
            .map(|b| b.trim().chars().any(|c| !c.is_whitespace()))
            .unwrap_or(false);
        if body_present {
            body_hits += 1;
        }

        // conventional commit heuristic
        const TYPES: [&str; 12] = [
            "feat", "fix", "chore", "refactor", "docs", "test", "perf", "style", "build", "ci",
            "revert", "deps",
        ];
        let is_conv = TYPES.iter().any(|t| {
            subject.starts_with(&format!("{t}:")) || subject.starts_with(&format!("{t}("))
        });
        if is_conv {
            conv_hits += 1;
        }

        commits_log.push((email, dt));
    }

    let total_commits = seen;
    let (first_date, last_date) = match (first_date, last_date) {
        (Some(f), Some(l)) => (f, l),
        _ => {
            let now = Local::now();
            (now, now)
        }
    };

    // Active period & average
    let active_days = (last_date.date_naive() - first_date.date_naive())
        .num_days()
        .max(0)
        + 1;
    let avg_commits_per_day = if active_days > 0 {
        total_commits as f64 / active_days as f64
    } else {
        0.0
    };

    // Peak day
    let peak_day = day_counts
        .iter()
        .max_by_key(|(_, c)| **c)
        .map(|(d, c)| (*d, *c));

    // Longest idle gap (sort ascending by date and compute max gap)
    all_dates.sort_unstable();
    let mut longest_idle_gap_days = 0i64;
    for w in all_dates.windows(2) {
        if let [a, b] = w {
            let gap = (*b - *a).num_days();
            if gap > longest_idle_gap_days {
                longest_idle_gap_days = gap;
            }
        }
    }

    // Weekday & work-hours %
    let work_hours_pct = if total_commits > 0 {
        100.0 * (work_hours_hits as f64) / (total_commits as f64)
    } else {
        0.0
    };

    // Merge/Revert rates
    let merge_rate = if total_commits > 0 {
        (merges as f64) * 100.0 / total_commits as f64
    } else {
        0.0
    };
    let revert_rate = if total_commits > 0 {
        (reverts as f64) * 100.0 / total_commits as f64
    } else {
        0.0
    };

    // Message stats
    let msg_median_len = if subj_lens.is_empty() {
        0
    } else {
        let mut v = subj_lens.clone();
        v.sort_unstable();
        v[v.len() / 2]
    };
    let msg_body_pct = if total_commits > 0 {
        100.0 * (body_hits as f64) / (total_commits as f64)
    } else {
        0.0
    };
    let conv_commit_pct = if total_commits > 0 {
        100.0 * (conv_hits as f64) / (total_commits as f64)
    } else {
        0.0
    };

    // Recent windows
    let mut active_last_90 = HashSet::<String>::new();
    let mut recent30_counts = HashMap::<String, usize>::new();
    let t90 = last_date - Duration::days(90);
    let t30 = last_date - Duration::days(30);
    let mut commits_last_90 = 0usize;

    for (email, dt) in &commits_log {
        if *dt >= t90 {
            active_last_90.insert(email.clone());
            commits_last_90 += 1;
        }
        if *dt >= t30 {
            *recent30_counts.entry(email.clone()).or_default() += 1;
        }
    }
    let momentum_90d_pct = if total_commits > 0 {
        100.0 * (commits_last_90 as f64) / (total_commits as f64)
    } else {
        0.0
    };
    let top_recent_30d = recent30_counts.into_iter().max_by_key(|(_, n)| *n);
    // --- NEW: Top authors in the last 12 months (365 days from last commit) ---
    let t365 = last_date - Duration::days(365);
    let mut recent12_map: HashMap<String, AuthorMeta> = HashMap::new();
    for (email, dt) in &commits_log {
        if *dt >= t365 {
            let e = recent12_map.entry(email.clone()).or_insert(AuthorMeta {
                count: 0,
                first: *dt,
                last: *dt,
            });
            e.count += 1;
            if *dt < e.first {
                e.first = *dt;
            }
            if *dt > e.last {
                e.last = *dt;
            }
        }
    }
    let recent12_total = recent12_map.values().map(|e| e.count).sum();
    let recent12_stats = CommitStats {
        total_seen: recent12_total,
        data: recent12_map,
    };

    // Drive-by & core size & concentration (HHI, Gini)
    let mut counts: Vec<(String, usize)> = data.iter().map(|(k, v)| (k.clone(), v.count)).collect();
    let contributors_total = counts.len();
    counts.sort_by(|a, b| b.1.cmp(&a.1));

    let drive_by_ratio = if contributors_total > 0 {
        let drive = counts.iter().filter(|(_, c)| *c <= 2).count();
        (drive as f64) * 100.0 / (contributors_total as f64)
    } else {
        0.0
    };

    let total_c: usize = counts.iter().map(|(_, c)| *c).sum();
    let hhi = if total_c > 0 {
        counts
            .iter()
            .map(|(_, c)| {
                let s = *c as f64 / total_c as f64;
                s * s
            })
            .sum::<f64>()
    } else {
        0.0
    };

    let gini = if total_c == 0 || contributors_total == 0 {
        0.0
    } else {
        let mut xs: Vec<usize> = counts.iter().map(|(_, c)| *c).collect();
        xs.sort_unstable();
        let n = xs.len() as f64;
        let sum: f64 = xs.iter().map(|&x| x as f64).sum();
        let mut cum = 0f64;
        let mut num = 0f64;
        for &x in &xs {
            cum += x as f64;
            num += cum;
        }
        if sum > 0.0 {
            (n + 1.0 - 2.0 * (num / sum)) / n
        } else {
            0.0
        }
    };

    // Core size: minimal top authors covering ≥80% of commits
    let mut acc = 0usize;
    let target = ((total_c as f64) * 0.80).ceil() as usize;
    let mut core_size_80pct = 0usize;
    for (_, c) in &counts {
        core_size_80pct += 1;
        acc += *c;
        if acc >= target {
            break;
        }
    }
    if total_c == 0 {
        core_size_80pct = 0;
    }

    let summary = RepoSummary {
        total_commits,
        contributors_total,
        first_author,
        first_date,
        last_author,
        last_date,
        active_days,
        avg_commits_per_day,
        peak_day,
        active_authors_last_90d: active_last_90.len(),
        top_recent_30d,

        drive_by_ratio,
        core_size_80pct,
        hhi,
        gini,
        longest_idle_gap_days,
        weekday_counts,
        work_hours_pct,
        merge_rate,
        revert_rate,
        msg_median_len,
        msg_body_pct,
        conv_commit_pct,
        momentum_90d_pct,
    };

    RepoScan {
        stats: CommitStats {
            total_seen: total_commits,
            data,
        },
        summary,
        recent12: recent12_stats,
    }
}
