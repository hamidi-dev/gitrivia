use anyhow::Result;
use clap::Args;
use serde_json::json;

use crate::commands::Global;
use crate::domain::{git::RepoExt, bus_factor};
use comfy_table::{Table, presets::UTF8_HORIZONTAL_ONLY, Cell, CellAlignment};

#[derive(Debug, Args)]
pub struct BusFactor {
    /// Path to the Git repo
    #[arg(short, long, default_value=".")]
    pub path: String,

    /// JSON output (overrides global --json)
    #[arg(long)]
    pub json: bool,

    /// Ownership % threshold (0.0..1.0), default 0.75
    #[arg(long, default_value="0.75")]
    pub threshold: f64,

    /// FAST mode: estimate ownership by "touch counts" (no blame)
    #[arg(long)]
    pub fast: bool,

    /// Only consider the last N commits in --fast mode (default: 5000, 0 = all)
    #[arg(long, default_value="5000")]
    pub max_commits: usize,

    /// Number of rows to print (for human output, 0 = unlimited)
    #[arg(long, default_value="0")]
    pub limit: usize,

    /// Number of threads for parallel blame (0 = rayon default)
    #[arg(long, default_value="0")]
    pub threads: usize,
}

impl super::Runnable for BusFactor {
    fn run(self, g: &Global) -> Result<()> {
        // Helper to render a nice table
        fn render_table(title: &str, unit: &str, rows: &[(String, String, f64, usize)], limit: usize) {
            let mut t = Table::new();
            t.load_preset(UTF8_HORIZONTAL_ONLY)
                .set_header(vec!["File", "Owner", "Ownership", unit]);

            let to_show = if limit == 0 { rows.len() } else { limit };
            for (file, owner, ratio, total) in rows.iter().take(to_show) {
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

        // Build a closure so we can install a rayon pool optionally
        let run_inner = || -> Result<()> {
            let repo = RepoExt::open(&self.path)?;

            // Compute scores once (vector), then split by threshold
            let (mode, unit, mut scores) = if self.fast {
                let max = if self.max_commits == 0 { None } else { Some(self.max_commits) };
                let s = bus_factor::compute_scores_fast(repo.repo(), max)?;
                ("FAST (touches)", "Touches", s)
            } else {
                let s = bus_factor::compute_scores_parallel(&self.path)?;
                ("Blame (lines)", "Lines", s)
            };

            let hits: Vec<_> = scores
                .iter()
                .filter(|s| s.ratio > self.threshold)
                .map(|s| (s.file.clone(), s.top_author.clone(), s.ratio, s.total))
                .collect();

            if g.json || self.json {
                let top_candidates = {
                    let iter = scores.iter().map(|s| json!({
                        "file": s.file,
                        "author": s.top_author,
                        "ownership": s.ratio,
                        "total": s.total
                    }));
                    if self.limit == 0 {
                        iter.collect::<Vec<_>>()
                    } else {
                        iter.take(self.limit).collect::<Vec<_>>()
                    }
                };

                let payload = json!({
                    "mode": mode,
                    "threshold": self.threshold,
                    "matches": hits.iter().map(|(f,a,r,t)| json!({
                        "file": f, "author": a, "ownership": r, "total": t
                    })).collect::<Vec<_>>(),
                    "top_candidates": top_candidates
                });

                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            // Human output as table
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
                    .map(|(f,a,r,t)| (f.clone(), a.clone(), *r, *t))
                    .collect();

                render_table(
                    &format!("⚠️  Bus Factor — Files above threshold {:>4.1}%  ({mode})", self.threshold * 100.0),
                    unit,
                    &rows,
                    self.limit
                );
            }

            Ok(())
        };

        if self.threads == 0 || self.fast {
            run_inner()
        } else {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(self.threads).build()?;
            pool.install(|| run_inner())
        }
    }
}

