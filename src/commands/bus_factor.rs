use anyhow::Result;
use clap::Args;
use serde_json::json;

use crate::commands::Global;
use crate::domain::{bus_factor, git::RepoExt};
use comfy_table::{presets::UTF8_HORIZONTAL_ONLY, Table};

#[derive(Debug, Args)]
pub struct BusFactor {
    #[arg(short, long, default_value = ".")]
    pub path: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long, default_value = "0.75")]
    pub threshold: f64,
    #[arg(long)]
    pub fast: bool,
    #[arg(long, default_value = "5000")]
    pub max_commits: usize,
    #[arg(long)]
    pub all: bool,
    #[arg(long, value_delimiter = ',')]
    pub include_ext: Vec<String>,
    #[arg(long, default_value = "10")]
    pub min_total: usize,
    #[arg(long, value_parser = ["file","dir"], default_value = "file")]
    pub by: String,
    #[arg(long, default_value = "2")]
    pub depth: usize,
    #[arg(long, default_value = "20")]
    pub limit: usize,
    #[arg(long, default_value = "0")]
    pub threads: usize,
}

impl super::Runnable for BusFactor {
    fn run(self, g: &Global) -> Result<()> {
        let json = self.json || g.json;

        fn render_table(
            title: &str,
            unit: &str,
            rows: &[(String, String, f64, usize)],
            limit: usize,
        ) {
            let mut t = Table::new();
            t.load_preset(UTF8_HORIZONTAL_ONLY).set_header(vec![
                if title.contains("Directory") {
                    "Directory"
                } else {
                    "File"
                },
                "Owner",
                "Ownership",
                unit,
            ]);

            for (k, owner, ratio, total) in rows.iter().take(limit) {
                t.add_row(vec![
                    k.clone(),
                    owner.clone(),
                    format!("{:>4.1}%", ratio * 100.0),
                    total.to_string(),
                ]);
            }
            println!("{title}");
            println!("{t}");
        }

        let opts = bus_factor::ScanOpts {
            all: self.all,
            include_ext: self.include_ext.clone(),
            min_total: self.min_total,
        };

        let run_inner = || -> Result<()> {
            let repo = RepoExt::open(&self.path)?;
            let (mode, unit) = if self.fast {
                ("FAST (touches)", "Touches")
            } else {
                ("Blame (lines)", "Lines")
            };

            if self.by == "dir" {
                if self.fast {
                    let max = if self.max_commits == 0 {
                        None
                    } else {
                        Some(self.max_commits)
                    };
                    let scores =
                        bus_factor::compute_dir_scores_fast(repo.repo(), max, &opts, self.depth)?;
                    let hits: Vec<_> = scores
                        .iter()
                        .filter(|s| s.ratio > self.threshold)
                        .map(|s| (s.dir.clone(), s.top_author.clone(), s.ratio, s.total))
                        .collect();

                    if json {
                        let payload = json!({
                            "mode": mode, "by": "dir", "depth": self.depth, "threshold": self.threshold,
                            "matches": hits.iter().map(|(d,a,r,t)| json!({"dir": d, "author": a, "ownership": r, "total": t})).collect::<Vec<_>>(),
                            "top_candidates": scores.iter().take(self.limit).map(|s| json!({"dir": s.dir, "author": s.top_author, "ownership": s.ratio, "total": s.total})).collect::<Vec<_>>()
                        });
                        println!("{}", serde_json::to_string_pretty(&payload)?);
                        return Ok(());
                    }

                    if hits.is_empty() {
                        println!(
                            "No directories exceed threshold {:>4.1}% — {} mode.\nTop candidates:",
                            self.threshold * 100.0,
                            mode
                        );
                        let rows: Vec<_> = scores
                            .iter()
                            .map(|s| (s.dir.clone(), s.top_author.clone(), s.ratio, s.total))
                            .collect();
                        render_table(
                            "⚠️  Bus Factor — Top Directories (candidates)",
                            unit,
                            &rows,
                            self.limit,
                        );
                    } else {
                        render_table(
                            &format!(
                                "⚠️  Bus Factor — Directories above threshold {:>4.1}%  ({mode})",
                                self.threshold * 100.0
                            ),
                            unit,
                            &hits,
                            self.limit,
                        );
                    }
                    return Ok(());
                } else {
                    let scores =
                        bus_factor::compute_dir_scores_parallel(&self.path, &opts, self.depth)?;
                    let hits: Vec<_> = scores
                        .iter()
                        .filter(|s| s.ratio > self.threshold)
                        .map(|s| (s.dir.clone(), s.top_author.clone(), s.ratio, s.total))
                        .collect();

                    if json {
                        let payload = json!({
                            "mode": mode, "by": "dir", "depth": self.depth, "threshold": self.threshold,
                            "matches": hits.iter().map(|(d,a,r,t)| json!({"dir": d, "author": a, "ownership": r, "total": t})).collect::<Vec<_>>(),
                            "top_candidates": scores.iter().take(self.limit).map(|s| json!({"dir": s.dir, "author": s.top_author, "ownership": s.ratio, "total": s.total})).collect::<Vec<_>>()
                        });
                        println!("{}", serde_json::to_string_pretty(&payload)?);
                        return Ok(());
                    }

                    if hits.is_empty() {
                        println!(
                            "No directories exceed threshold {:>4.1}% — {} mode.\nTop candidates:",
                            self.threshold * 100.0,
                            mode
                        );
                        let rows: Vec<_> = scores
                            .iter()
                            .map(|s| (s.dir.clone(), s.top_author.clone(), s.ratio, s.total))
                            .collect();
                        render_table(
                            "⚠️  Bus Factor — Top Directories (candidates)",
                            unit,
                            &rows,
                            self.limit,
                        );
                    } else {
                        render_table(
                            &format!(
                                "⚠️  Bus Factor — Directories above threshold {:>4.1}%  ({mode})",
                                self.threshold * 100.0
                            ),
                            unit,
                            &hits,
                            self.limit,
                        );
                    }
                    return Ok(());
                }
            }

            // by == "file"
            if self.fast {
                let max = if self.max_commits == 0 {
                    None
                } else {
                    Some(self.max_commits)
                };
                let scores = bus_factor::compute_scores_fast(repo.repo(), max, &opts)?;
                let hits: Vec<_> = scores
                    .iter()
                    .filter(|s| s.ratio > self.threshold)
                    .map(|s| (s.file.clone(), s.top_author.clone(), s.ratio, s.total))
                    .collect();

                if json {
                    let payload = json!({
                        "mode": mode, "by": "file", "threshold": self.threshold,
                        "matches": hits.iter().map(|(f,a,r,t)| json!({"file": f, "author": a, "ownership": r, "total": t})).collect::<Vec<_>>(),
                        "top_candidates": scores.iter().take(self.limit).map(|s| json!({"file": s.file, "author": s.top_author, "ownership": s.ratio, "total": s.total})).collect::<Vec<_>>()
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                    return Ok(());
                }

                if hits.is_empty() {
                    println!(
                        "No files exceed threshold {:>4.1}% — {} mode.\nTop candidates:",
                        self.threshold * 100.0,
                        mode
                    );
                    let rows: Vec<_> = scores
                        .iter()
                        .map(|s| (s.file.clone(), s.top_author.clone(), s.ratio, s.total))
                        .collect();
                    render_table("⚠️  Bus Factor — Top Candidates", unit, &rows, self.limit);
                } else {
                    render_table(
                        &format!(
                            "⚠️  Bus Factor — Files above threshold {:>4.1}%  ({mode})",
                            self.threshold * 100.0
                        ),
                        unit,
                        &hits,
                        self.limit,
                    );
                }
                return Ok(());
            } else {
                let scores = bus_factor::compute_scores_parallel(&self.path, &opts)?;
                let hits: Vec<_> = scores
                    .iter()
                    .filter(|s| s.ratio > self.threshold)
                    .map(|s| (s.file.clone(), s.top_author.clone(), s.ratio, s.total))
                    .collect();

                if json {
                    let payload = json!({
                        "mode": mode, "by": "file", "threshold": self.threshold,
                        "matches": hits.iter().map(|(f,a,r,t)| json!({"file": f, "author": a, "ownership": r, "total": t})).collect::<Vec<_>>(),
                        "top_candidates": scores.iter().take(self.limit).map(|s| json!({"file": s.file, "author": s.top_author, "ownership": s.ratio, "total": s.total})).collect::<Vec<_>>()
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                    return Ok(());
                }

                if hits.is_empty() {
                    println!(
                        "No files exceed threshold {:>4.1}% — {} mode.\nTop candidates:",
                        self.threshold * 100.0,
                        mode
                    );
                    let rows: Vec<_> = scores
                        .iter()
                        .map(|s| (s.file.clone(), s.top_author.clone(), s.ratio, s.total))
                        .collect();
                    render_table("⚠️  Bus Factor — Top Candidates", unit, &rows, self.limit);
                } else {
                    render_table(
                        &format!(
                            "⚠️  Bus Factor — Files above threshold {:>4.1}%  ({mode})",
                            self.threshold * 100.0
                        ),
                        unit,
                        &hits,
                        self.limit,
                    );
                }
                return Ok(());
            }
        };

        if !self.fast && self.threads > 0 {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(self.threads)
                .build()?;
            pool.install(|| run_inner())
        } else {
            run_inner()
        }
    }
}
