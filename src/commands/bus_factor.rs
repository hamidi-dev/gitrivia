use anyhow::Result;
use clap::Args;
use serde_json::json;

use crate::commands::Global;
use crate::domain::{bus_factor, git::RepoExt};
use comfy_table::{presets::UTF8_HORIZONTAL_ONLY, Table};

#[derive(Debug, Args)]
pub struct BusFactor {
    /// Path to the Git repo
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// JSON output (overrides global --json)
    #[arg(long)]
    pub json: bool,

    /// Ownership % threshold (0.0..1.0), default 0.75
    #[arg(long, default_value = "0.75")]
    pub threshold: f64,

    /// FAST mode: estimate ownership by "touch counts" (no blame)
    #[arg(long)]
    pub fast: bool,

    /// Only consider the last N commits in --fast mode (0 = all)
    #[arg(long, default_value = "5000")]
    pub max_commits: usize,

    /// Include ALL tracked files (ignore extension filter)
    #[arg(long)]
    pub all: bool,

    /// Extra extensions to include (comma-separated), e.g. "lua,vim,conf"
    #[arg(long, value_delimiter = ',')]
    pub include_ext: Vec<String>,

    /// Minimum total (lines for blame / touches for fast) to report
    #[arg(long, default_value = "10")]
    pub min_total: usize,

    /// Number of rows to print (human output)
    #[arg(long, default_value = "20")]
    pub limit: usize,

    /// Number of threads for parallel blame (0 = rayon default)
    #[arg(long, default_value = "0")]
    pub threads: usize,
}

impl super::Runnable for BusFactor {
    fn run(self, g: &Global) -> Result<()> {
        fn render_table(title: &str, unit: &str, rows: &[(String, String, f64, usize)], limit: usize) {
            let mut t = Table::new();
            t.load_preset(UTF8_HORIZONTAL_ONLY)
                .set_header(vec!["File", "Owner", "Ownership", unit]);

            for (file, owner, ratio, total) in rows.iter().take(limit) {
                t.add_row(vec![
                    file.clone(),
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

            let (mode, unit, mut scores) = if self.fast {
                let max = if self.max_commits == 0 {
                    None
                } else {
                    Some(self.max_commits)
                };
                let s = bus_factor::compute_scores_fast(repo.repo(), max, &opts)?;
                ("FAST (touches)", "Touches", s)
            } else {
                let s = bus_factor::compute_scores_parallel(&self.path, &opts)?;
                ("Blame (lines)", "Lines", s)
            };

            let hits: Vec<_> = scores
                .iter()
                .filter(|s| s.ratio > self.threshold)
                .map(|s| (s.file.clone(), s.top_author.clone(), s.ratio, s.total))
                .collect();

            if g.json || self.json {
                let payload = json!({
                    "mode": mode,
                    "threshold": self.threshold,
                    "matches": hits.iter().map(|(f,a,r,t)| json!({
                        "file": f, "author": a, "ownership": r, "total": t
                    })).collect::<Vec<_>>(),
                    "top_candidates": scores.iter().take(self.limit).map(|s| json!({
                        "file": s.file, "author": s.top_author,
                        "ownership": s.ratio, "total": s.total
                    })).collect::<Vec<_>>()
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
                let rows: Vec<_> = hits
                    .iter()
                    .map(|(f, a, r, t)| (f.clone(), a.clone(), *r, *t))
                    .collect();
                render_table(
                    &format!(
                        "⚠️  Bus Factor — Files above threshold {:>4.1}%  ({mode})",
                        self.threshold * 100.0
                    ),
                    unit,
                    &rows,
                    self.limit,
                );
            }

            Ok(())
        };

        // Only control threads in blame mode; fast mode is single-threaded revwalk
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

