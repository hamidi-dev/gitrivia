# 🧠 `gitrivia`

A fast Rust CLI to explore who did what, when, and how much in any Git repo.  
Great for engineers, tech leads, or curious code archaeologists 🧬.

> Designed to handle **large repos** (think Linux-kernel scale) with a single revwalk and minimal allocations.

---

## 🚀 Install

```bash
cargo build --release
cp target/release/gitrivia /usr/local/bin/gitrivia   # or put it on your PATH however you like
```

## ⚙️ Global flags

These work with every subcommand:

- `--json` → machine-readable output (for scripts, dashboards, LLMs)
- `--desc` → sort descending when applicable (e.g., author lists)

Examples:

```bash
gitrivia --json stats
gitrivia --desc top-authors --since 2024-01-01
```

## 📊 Commands

### 🔹 stats

A high-signal overview of the repo + Top-5 contributors.

```bash
gitrivia stats
```

Example (human):

```yaml
✨ Repo summary
  First commit:     2013-03-18 by nick@nickdownie.com
  Last commit:      2025-01-21 by ethan.shea1@gmail.com
  Total commits:    4,545
  Contributors:     533
  Active period:    4,328 days
  Avg commits/day:  1.05
  Peak day:         2016-02-28 (37 commits)
  Longest idle gap: 41 days (largest pause between commits)
  Momentum (90d):   4.3% of all commits, 9 authors active
  Top last 30d:     mariss@mariss.no (3 commits)

👥 Contributors
  Drive-by ratio:   62%  (share of authors with ≤2 commits; higher → many one-offs)
  Core size (80%):  14   (minimal number of authors covering 80% of commits)
  Concentration:    HHI 0.21  |  Gini 0.78  (higher → more concentrated)

⏰ Activity patterns
  Weekdays: Mon 18.0% Tue 17.0% Wed 16.0% Thu 17.0% Fri 20.0% Sat 6.0% Sun 6.0%
  Work-hours (09–18): 72%

🔀 Merge/Revert
  Merge rate:  31%   Revert rate: 1.8%

📝 Messages
  Median subject length: 48 chars
  With body:             63%
  Conventional commits:  54%

🔥 Top 5 authors:
───────────────────────────────────────────────────────────────────────────────────
 Author                                          Commits   First        Last
═══════════════════════════════════════════════════════════════════════════════════
 alice@example.com                               1012      2014-01-02   2025-01-15
 bob@example.com                                 740       2016-06-12   2024-11-02
 charlie@example.com                             388       2015-09-01   2023-12-12
 dana@example.org                                212       2017-08-22   2022-05-30
 eve@users.noreply.github.com                    201       2018-03-05   2020-10-19
───────────────────────────────────────────────────────────────────────────────────

Legend:
  Drive-by ratio = Authors with ≤2 commits (higher → many one-off contributors).
  Core size (80%) = Minimal number of authors required to cover 80% of commits.
  HHI/Gini = Contribution concentration (higher → more dominated by few people).
```

Same, but JSON (truncated):

```bash
gitrivia --json stats | jq
```

```json
{
  "summary": {
    "first_commit": { "date": "2013-03-18", "author": "nick@nickdownie.com" },
    "last_commit":  { "date": "2025-01-21", "author": "ethan.shea1@gmail.com" },
    "total_commits": 4545,
    "contributors_total": 533,
    "active_days": 4328,
    "avg_commits_per_day": 1.05,
    "peak_day": { "date": "2016-02-28", "commits": 37 },
    "longest_idle_gap_days": 41,
    "momentum_90d_pct": 4.3,
    "active_authors_last_90d": 9,
    "contributors": {
      "drive_by_ratio_pct": 62.0,
      "core_size_80pct": 14,
      "concentration_hhi": 0.21,
      "concentration_gini": 0.78
    },
    "activity_patterns": {
      "weekday_counts_mon_sun": [823, 777, 734, 772, 908, 275, 256],
      "work_hours_pct_9_18": 72.0
    },
    "merge_revert": { "merge_rate_pct": 31.0, "revert_rate_pct": 1.8 },
    "messages": {
      "median_subject_len": 48,
      "body_present_pct": 63.0,
      "conventional_commit_pct": 54.0
    },
    "top_recent_30d": { "author": "mariss@mariss.no", "commits": 3 }
  },
  "top_5_authors": [
    { "email": "alice@example.com", "count": 1012, "first": "2014-01-02", "last": "2025-01-15" },
    { "email": "bob@example.com",   "count": 740,  "first": "2016-06-12", "last": "2024-11-02" }
  ]
}
```

## Tips

- `--limit N` processes only the most recent N commits (faster on huge repos).
- Use `--desc` to sort descending for other commands; stats Top-5 is always desc.

### 🔹 top-authors

Show per-author counts since a date (good for current period leadership).

```bash
gitrivia top-authors --since 2024-01-01 --desc
```

Example:

```yaml
Authors since Some(2024-01-01):
alice@example.com                  120 commits   🗓  2024-01-02 → 2025-01-15
bob@example.com                     92 commits   🗓  2024-02-10 → 2024-11-02
...
```

### 🔹 author-activity

First + last commit dates and total count for one author (exact email match).

```bash
gitrivia author-activity --author alice@example.com
```

```yaml
alice@example.com                  1012 commits 🗓  2014-01-02 → 2025-01-15
```

## 🧠 Deep Dives

### 🔸 blame-summary

Who owns how many lines of a file (via git blame).

```bash
gitrivia blame-summary --file src/main.rs
# or JSON
gitrivia blame-summary --file src/main.rs --json | jq
```

Output:

```text
alice@example.com                  210 lines
bob@example.com                    154 lines
...
```

### 🔸 file-contributions

Heatmap of which authors touched which files (counting commits that changed a file).

```bash
gitrivia file-contributions [--json]
```

Example:

```text
src/lib.rs
  alice@example.com                 12 commits
  bob@example.com                    7 commits
src/main.rs
  alice@example.com                  5 commits
```

### 🔸 commit-times

When do people commit? Night owls vs office coders. (night, morning, afternoon, evening)

```bash
gitrivia commit-times [--json]
```

```text
alice@example.com
  morning    84
  afternoon  210
  evening     73
  night       18
```

### 🔸 first-commits

The oldest commit per author.

```bash
gitrivia first-commits [--json]
```

```text
alice@example.com                  2014-01-02
bob@example.com                    2016-06-12
...
```

### 🔸 top-coauthors

Find dev pairs that work on the same files — great for org charts or refactoring ownership.

```bash
gitrivia top-coauthors [--json]
```

```text
alice@example.com + bob@example.com                      42 shared files
alice@example.com + charlie@example.com                  17 shared files
```

### 🔸 bus-factor

🚍 Warns about files dangerously dominated by a single author.

```bash
gitrivia bus-factor [--threshold 0.75] [--json]
```

Threshold defaults to 0.75

Looks at all tracked source files

Example:

```text
⚠️  src/payment.rs                94.5% by mo@example.com
```

## ✨ JSON Everywhere

Every command supports `--json` for automation or AI analysis:

```bash
gitrivia top-coauthors --json | jq
```

## 📦 Roadmap

- `--since`, `--until` filtering on all commands
- TUI mode (interactive dashboard)
- Churn detection
- AI-assisted output
- Per-author "streaks" and "bus factor by directory"

## 🧮 Metric cheat-sheet

- **Drive-by ratio** — % of authors with ≤2 commits (higher → many one-offs).
- **Core size (80%)** — minimal number of top authors that account for 80% of commits.
- **HHI / Gini** — concentration of contributions (higher → dominated by few).
- **Longest idle gap** — largest pause between two consecutive commits.
- **Momentum (90d)** — % of lifetime commits that occurred in the last 90 days.
- **Work-hours %** — share of commits between 09:00–17:59 local time.
- **Merge / Revert rate** — fraction of merge / revert commits; proxy for PR volume and churn.
- **Message hygiene** — subject median length; % with non-empty body; % Conventional Commits.

## 🦀 Built With

- git2
- chrono
- serde
- Rust. Obviously.

PRs welcome. Or don't. I'm not your boss. 😎
