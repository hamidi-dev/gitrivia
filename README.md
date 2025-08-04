# 🧠 gitrivia

![stats](./screenshots/stats.jpeg) 

A fast Rust CLI to explore **who did what, when, and how much** in any Git repo.
Great for engineers, tech leads, and curious code archaeologists 🧬.

> Built for **large repos** (Linux‑kernel scale): one revwalk, minimal allocations,
> optional parallel blame, and fast heuristics when you need them.

---

## 🚀 Install

```bash
cargo build --release
cp target/release/gitrivia /usr/local/bin/gitrivia   # or anywhere on your PATH
```

---

## ⚙️ Global flags

Place **before** the subcommand; apply to every command.

* `--json` → machine‑readable output (scripts/dashboards/LLMs)
* `--desc` → descending sort where applicable (e.g., author lists)

Examples:

```bash
gitrivia --json stats
gitrivia --desc top-authors --since 2025-01-01
```

---

## 🧭 Which command should I run?

> Start with a question, follow the arrow, run the command.

* **I want a quick *health snapshot* of the repo.**

  * → `gitrivia stats`
* **Who’s most active *right now* (this quarter, etc.)?**

  * → `gitrivia top-authors --since YYYY-MM-DD`
* **Show activity range for a *single author*.**

  * → `gitrivia author-activity --author EMAIL`
* **Who *owns* this file’s lines of code?**

  * → `gitrivia blame-summary --file PATH`
* **Which files are touched by which authors (heatmap)?**

  * → `gitrivia file-contributions`
* **When do people commit (night owls vs office hours)?**

  * → `gitrivia commit-times`
* **Who started contributing when (first commit per author)?**

  * → `gitrivia first-commits`
* **Who tends to work together (shared files)?**

  * → `gitrivia top-coauthors`
* **Where’s the *ownership risk* (single‑author dominance)?**

  * Accuracy (line ownership, slower):

    * Files → `gitrivia bus-factor --threshold 0.75`
    * Dirs  → `gitrivia bus-factor --by dir --depth 2 --threshold 0.75`
  * Speed (recent touches, heuristic):

    * Files → `gitrivia bus-factor --fast --max-commits 5000 --threshold 0.7`
    * Dirs  → `gitrivia bus-factor --by dir --fast --depth 2 --max-commits 5000 --threshold 0.7`
* **What are the *hotspots* lately (volatile files/dirs)?**

  * Files → `gitrivia churn --window-days 90`
  * Dirs  → `gitrivia churn --by dir --depth 2 --window-days 90`

### 🍫 Quick cheat‑sheet

| Goal / Question                           | Command                                                | Tip                                   |
| ----------------------------------------- | ------------------------------------------------------ | ------------------------------------- |
| Snapshot repo health & Top‑5 contributors | `gitrivia stats`                                       | Add `--json` for dashboards           |
| Current period leaders                    | `gitrivia top-authors --since 2025-01-01 --desc`       | `--desc` sorts top first              |
| One author’s activity range               | `gitrivia author-activity --author alice@example.com`  | Exact email match                     |
| Who owns this file                        | `gitrivia blame-summary --file src/main.rs`            | Use before risky refactors            |
| File ↔ author heatmap                     | `gitrivia file-contributions`                          | Good for PR routing                   |
| Commit time distribution                  | `gitrivia commit-times`                                | Time‑zone coordination                |
| First commit per author                   | `gitrivia first-commits`                               | Find long‑term maintainers            |
| Frequent co‑workers (shared files)        | `gitrivia top-coauthors`                               | Pairing & knowledge transfer          |
| Bus factor (accurate, blame) — files/dirs | `gitrivia bus-factor [--by dir --depth 2]`             | Add `--threads N` for speed           |
| Bus factor (fast, touches) — files/dirs   | `gitrivia bus-factor --fast [--by dir --depth 2]`      | Tune `--max-commits` (recent history) |
| Recent hotspots (churn) — files/dirs      | `gitrivia churn [--by dir --depth 2] --window-days 60` | Larger window = smoother trends       |

---

## 📊 Commands

### 🔹 `stats` — high‑signal overview

**What:** Summarizes repo health + patterns and shows **Top‑5 contributors**.

**Why:** Due diligence, onboarding, health checks.

```bash
gitrivia stats
# add --json for dashboards
```

**Example (human):**

```text
✨ Repo summary
  First commit:     2013-03-18 by alice@alice.com
  Last commit:      2025-01-21 by bob@bob.com
  Total commits:    4,545
  Contributors:     533
  Active period:    4,328 days
  Avg commits/day:  1.05
  Peak day:         2016-02-28 (37 commits)
  Longest idle gap: 41 days (largest pause between commits)
  Momentum (90d):   4.3% of all commits, 9 authors active
  Top last 30d:     john@doe.com (3 commits)

👥 Contributors
  Drive-by ratio:   62%
  Core size (80%):  14
  Concentration:    HHI 0.21  |  Gini 0.78

⏰ Activity patterns
  Weekdays: Mon 18% Tue 17% Wed 16% Thu 17% Fri 20% Sat 6% Sun 6%
  Work-hours (09–18): 72%

🔀 Merge/Revert
  Merge rate: 31%   Revert rate: 1.8%

📝 Messages
  Median subject length: 48 chars
  With body:             63%
  Conventional commits:  54%

🔥 Top 5 authors: (table)
```

**Tips**

* Use `--limit N` to scan only the newest commits.
* Top‑5 table is always **desc** by commits.

---

### 🔹 `top-authors` — who’s active since a date

**What:** Commit counts per author **since YYYY‑MM‑DD**.

**Why:** Quarterly/OKR reviews, current maintainers.

```bash
gitrivia top-authors --since 2025-01-01 --desc
```

---

### 🔹 `author-activity` — one author’s range

**What:** First + last commit dates and total count for an exact email.

**Why:** Validate ownership/tenure, find stale contributors.

```bash
gitrivia author-activity --author alice@example.com
```

---

### 🔸 `blame-summary` — line ownership for a file

**What:** Who owns how many lines (via `git blame`).

**Why:** Code owners, review routing, bus‑factor checks.

```bash
gitrivia blame-summary --file src/main.rs      # human
gitrivia blame-summary --file src/main.rs --json | jq
```

---

### 🔸 `file-contributions` — file ↔ author heatmap

**What:** Per file, how many commits each author made that changed it.

**Why:** Rough ownership/touch map; useful for refactors & PR routing.

```bash
gitrivia file-contributions [--json]
```

---

### 🔸 `commit-times` — when people commit

**What:** Buckets per author: `night` (00–05), `morning` (06–11), `afternoon` (12–17), `evening` (18–23).

**Why:** Coordination across time zones; after‑hours patterns.

```bash
gitrivia commit-times [--json]
```

---

### 🔸 `first-commits` — first commit per author

**What:** Oldest commit per author.

**Why:** Identify founding contributors / long‑term maintainers.

```bash
gitrivia first-commits [--json]
```

---

### 🔸 `top-coauthors` — frequent pairs

**What:** Contributor pairs that often modify the same files.

**Why:** Org maps, pairing opportunities, hidden silos.

```bash
gitrivia top-coauthors [--json]
```

---

### 🔸 `bus-factor` — risky ownership concentration (file/dir)

**What:** Flags files or directories dominated by a single author.

**Why:** Reduce risk; plan rotations, docs, or reviews.

**Modes**

* **Accurate (`blame`)**: line ownership (slower, parallelizable)
* **Fast (`--fast`)**: heuristic using **touch counts** from recent commits

**Directory aggregation** with `--by dir` and `--depth N`.

```bash
# Accurate (blame-based), files
gitrivia bus-factor --threshold 0.75

# Directory-level, accurate, depth=2
gitrivia bus-factor --by dir --depth 2 --threshold 0.7

# FAST heuristic (touches), last 5000 commits
gitrivia bus-factor --fast --max-commits 5000 --threshold 0.7
```

**Useful options**

* `--threshold 0.75`      : flag ownership ratio (default 0.75)
* `--by file|dir` `--depth N` : aggregate to directories
* `--threads N`           : parallel blame (accurate mode)
* `--all` / `--include-ext lua,vim` : file filtering
* `--min-total 25`        : skip tiny files (lines or touches)
* `--limit 20`            : rows to print (human output)

---

### 🔸 `churn` — recent volatility (file/dir)

**What:** Ranks files (or directories) by **weighted recent change**.
For each commit in the window (default **90 days**): compute `adds + dels`,
weight by linear decay (newer changes count more), then sum per path.

**Why:** Find hotspots, refactor targets, and risky entry points.

```bash
# Top volatile files in last 90 days
gitrivia churn --window-days 90

# Aggregate to directories (depth 2)
gitrivia churn --by dir --depth 2 --window-days 60
```

**Columns**

* `Churn`  : weighted (adds+dels)
* `Adds`   : added lines
* `Dels`   : deleted lines
* `Touches`: commits that touched this path in the window

**Filters**

* `--all` or `--include-ext lua,vim`
* `--min-total 1` to skip near‑empty paths
* `--limit 20` rows

**Interpretation**

* High `Churn` + high `Touches` → unstable hotspot; expect bugs/rework.
* High `Churn` + low `Touches` → big rewrites; verify test coverage & reviews.

---

## 🧪 Examples (copy/paste)

```bash
# Repo snapshot
gitrivia stats

# Current period leaders
gitrivia --desc top-authors --since 2025-01-01

# File ownership
gitrivia blame-summary --file src/main.rs

# Risky directories (accurate)
gitrivia bus-factor --by dir --depth 2 --threshold 0.7 --threads 8

# Fast triage (touches only)
gitrivia bus-factor --fast --max-commits 10000 --threshold 0.65

# Hotspots in last 60 days
gitrivia churn --window-days 60 --limit 30

# JSON for dashboards
gitrivia --json stats | jq
gitrivia churn --by dir --json | jq
```

---

## ✨ JSON everywhere

Every command accepts `--json` so you can feed dashboards and scripts:

```bash
gitrivia top-coauthors --json | jq
```

---

## 🧮 Metric cheat‑sheet

These are the metrics you’ll see in `gitrivia stats`. Each item includes:
**What it is**, **how we compute it**, **how to read it**, and **things to watch out for**.

> Notation used below:
>
> * *commits(author)* = number of commits by that author
> * *total\_commits* = total commits in the repo
> * *share(author)* = commits(author) / total\_commits

---

### Drive‑by ratio

**What:** How many contributors made only a tiny number of commits.

**Formula:**

```
(#authors with ≤ 2 commits / total authors) × 100
```

**Read it:**

* **High** → lots of one‑off or occasional contributors (e.g., quick fixes).
* **Low**  → a stable core team contributing repeatedly.

**Example:** If 20 out of 50 authors have ≤2 commits → 40%.

**Watch out:** Bot accounts or email aliases can skew this. Consider filtering bots.

---

### Core size (80%)

**What:** Minimum number of top contributors who together produce at least 80% of all commits.

**How:** Sort authors by commits (desc), then accumulate until you reach ≥ 80% of *total\_commits*; count how many authors that took.

**Read it:**

* **Small** core size → very concentrated work (few people do most of it).
* **Large** core size → more distributed contributions.

**Example:** If the top 6 authors cover ≥80% of commits, core size (80%) = 6.

**Watch out:** Commit count ≠ effort/LOC; it’s a good proxy but not perfect.

---

### HHI (Herfindahl‑Hirschman Index) / Gini

**What:** Both measure **concentration** of contributions across authors.

**Formulas:**

* `share(author) = commits(author) / total_commits`
* **HHI:** `Σ share(author)²` (sums over all authors). Range ≈ **1/N … 1**.

  * Closer to **1** → a single dominant contributor.
  * Closer to **1/N** → evenly spread across N authors.
* **Gini:** standard inequality index on the commit count distribution. Range **0 … 1**.

  * **0** → perfectly equal (everyone contributes the same number of commits).
  * **1** → perfectly unequal (one person does everything).

**Read it:** Higher HHI/Gini → more concentrated ownership.

**Example:** If two authors split 50/50, HHI = 0.5² + 0.5² = **0.50**; Gini is low.

**Watch out:** Based on **counts**, not lines/complexity; still very helpful at a glance.

---

### Longest idle gap

**What:** The longest pause (in days) between two consecutive commits.

**How:** Sort commit **dates** and compute the largest day‑to‑day gap.

**Read it:** Big numbers hint at long lulls (e.g., pre‑release freeze, repo abandonment).

**Example:** If the largest gap between any two commit dates is 41 days → **41**.

**Watch out:** Multiple commits on the same day don’t affect the max gap.

---

### Momentum (90d)

**What:** How much of the repo’s lifetime work happened **recently**.

**Formula:**

```
(commits in last 90 days / total_commits) × 100
```

**Read it:**

* **High** → the project is very active right now.
* **Low**  → most work happened in the past.

**Example:** If 200 of 2,000 commits are from the last 90d → 10%.

**Watch out:** Uses the repo’s **latest commit timestamp** as “now”. Old repos with no recent work will show low momentum by design.

---

### Work‑hours %

**What:** Share of commits made during **09:00–17:59** *local time of the machine running gitrivia*.

**Formula:**

```
(commits with local_time in 09:00–17:59 / total_commits) × 100
```

**Read it:** Cultural/process signal (office hours vs. evenings/weekends).

**Example:** If 720 of 1,000 commits fall in 09–17:59 → **72%**.

**Watch out:** Author machines might have wrong clocks; time zone is **your local** machine, not the contributor’s.

---

### Churn (windowed)

**What:** Measures how much code is **changing recently** (hotspots).

**Formula (per file/dir):**

```
Σ over commits in window: (adds + dels) × weight
```

Where **weight** decays linearly from 1.0 (newest) to \~0.0 (oldest in window).

**Read it:**

* **High churn + many touches** → unstable area, likely to need attention.
* **High churn + few touches** → large rewrites; check tests/review coverage.

**Example:** A file changed 10, 20, and 30 lines across three recent commits → base = 60; weighted by recency you might see \~45–55 depending on dates.

**Watch out:** For speed, churn does **not** enable rename detection by default—big renames can look like add+delete. You can make this configurable.

---

### Bus‑factor (per‑path dominance)

**What:** How concentrated ownership is for a file/dir (risk if one person dominates).

**Definition:** `max(author_share)` for that path.

**Two modes:**

* **Accurate (blame):**

  * `author_share = lines_owned(author) / total_lines`
  * Pros: line‑accurate; Cons: slower (can be parallelized).
* **FAST (touches):**

  * `author_share = touches(author) / total_touches` (commit‑level changes)
  * Pros: very quick; Cons: heuristic (recent bursts can dominate).

**Read it:**

* Values near **1.0** → single‑owner risk; spread‑out values → healthier.

**Example:** If Alice owns 780/1,000 lines → 0.78 (78%). With touches, if Alice made 39 of 50 touches → 0.78 as well.

**Watch out:**

* Accurate mode can flag vendor/lock files—use extension filters (`--all` / `--include-ext`).
* FAST mode is recency‑biased; great for triage, not for compliance.

---

## 📦 Roadmap

* Global `--since` / `--until` on all commands
* TUI dashboard
* Per‑author “streaks”
* PR‑level stats (merge latency, review load)
* Ownership diffs over time

---

## 🦀 Built with

* [git2](https://crates.io/crates/git2)
* [chrono](https://crates.io/crates/chrono)
* [serde](https://crates.io/crates/serde)
* Rust. Obviously.

PRs welcome :)

