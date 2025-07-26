# 🧠 `gitrivia`

A blazingly fast™ Rust CLI to explore who did what, when, and how much in any Git repo.
Perfect for engineers, tech leads, or curious archaeologists 🧬.
Handles even the Linux kernel's 1.3 million commits like it’s a weekend hobby.

---

## 🚀 Install

```bash
cargo build --release
cp target/release/gitrivia /usr/local/bin/gitrivia
```

---

## 📊 Commands

### 🔹 `stats`
Get total commits and per-author contribution (sorted ascending by default)

```bash
gitrivia stats --limit 500
```

Options:
- `--sort-desc`: show top authors first

---

### 🔹 `top-authors`
Show commit counts per author **since a given date**

```bash
gitrivia top-authors --since 2023-01-01
```

---

### 🔹 `author-activity`
Get first + last commit date and commit count for one author

```bash
gitrivia author-activity --author mo@example.com
```

---

## 🧠 Deep Dives

### 🔸 `blame-summary`
Show who owns how many lines in a file.

```bash
gitrivia blame-summary --file src/main.rs
```

Optional:
- `--json` for machine-readable output

---

### 🔸 `file-contributions`
Heatmap of which authors touched which files.

```bash
gitrivia file-contributions [--json]
```

---

### 🔸 `commit-times`
See who commits when: night owls vs office coders.

```bash
gitrivia commit-times [--json]
```

Time buckets:
- `night` (00–05)
- `morning` (06–11)
- `afternoon` (12–17)
- `evening` (18–23)

---

### 🔸 `first-commits`
Find the oldest commit per author.

```bash
gitrivia first-commits [--json]
```

---

### 🔸 `top-coauthors`
Find dev pairs that work on the same files — great for org charts or refactoring ownership.

```bash
gitrivia top-coauthors [--json]
```

---

### 🔸 `bus-factor`
🚍 Warns about files dangerously dominated by a single author.

```bash
gitrivia bus-factor [--threshold 0.75] [--json]
```

- Threshold defaults to `0.75`
- Looks at all tracked source files

Example output:
```
⚠️  src/payment.rs                94.5% by mo@example.com
```

---

## 🧪 Example Usage

```bash
gitrivia stats --limit 1000 --sort-desc
gitrivia blame-summary --file src/main.rs --json
gitrivia bus-factor --threshold 0.9
```

---

## ✨ JSON Everywhere

Every command supports `--json` for automation or AI analysis:

```bash
gitrivia top-coauthors --json | jq
```

---

## 📦 Roadmap

- [ ] `--since`, `--until` filtering on all commands
- [ ] TUI mode (interactive dashboard)
- [ ] Churn detection
- [ ] AI-assisted output

---

## 🦀 Built With

- [`git2`](https://crates.io/crates/git2)
- [`chrono`](https://crates.io/crates/chrono)
- [`serde`](https://crates.io/crates/serde)
- Rust. Obviously.

---

PRs welcome. Or don’t. I’m not your boss. 😎

