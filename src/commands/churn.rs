use anyhow::Result;
use clap::Args;
use comfy_table::{presets::UTF8_HORIZONTAL_ONLY, Table};
use serde_json::json;

use crate::commands::Global;
use crate::domain::git::RepoExt;
use crate::domain::{bus_factor::ScanOpts, churn};

#[derive(Debug, Args)]
pub struct Churn {
    #[arg(short, long, default_value = ".")]
    pub path: String,
    #[arg(long, default_value = "90")]
    pub window_days: i64,
    #[arg(long, value_parser = ["file","dir"], default_value = "file")]
    pub by: String,
    #[arg(long, default_value = "2")]
    pub depth: usize,
    #[arg(long)]
    pub all: bool,
    #[arg(long, value_delimiter = ',')]
    pub include_ext: Vec<String>,
    #[arg(long, default_value = "1")]
    pub min_total: usize,
    #[arg(long, default_value = "20")]
    pub limit: usize,
    #[arg(long)]
    pub json: bool,
}

impl super::Runnable for Churn {
    fn run(self, g: &Global) -> Result<()> {
        let json = self.json || g.json;

        let repo = RepoExt::open(&self.path)?;
        let opts = ScanOpts {
            all: self.all,
            include_ext: self.include_ext.clone(),
            min_total: self.min_total,
        };
        let mut entries = churn::compute_churn(repo.repo(), self.window_days, &opts)?;

        if self.by == "dir" {
            use std::collections::HashMap;
            let mut by_dir: HashMap<String, (f64, usize, usize, usize)> = HashMap::new();
            for e in entries.iter() {
                let k = churn::dir_key(&e.path, self.depth);
                let v = by_dir.entry(k).or_insert((0.0, 0, 0, 0));
                v.0 += e.churn;
                v.1 += e.adds;
                v.2 += e.dels;
                v.3 += e.touches;
            }
            let mut dir_rows: Vec<_> = by_dir
                .into_iter()
                .map(|(dir, (churn, adds, dels, touches))| (dir, churn, adds, dels, touches))
                .collect();
            dir_rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            if json {
                let payload = json!({
                    "by": "dir",
                    "depth": self.depth,
                    "window_days": self.window_days,
                    "rows": dir_rows.iter().take(self.limit).map(|(dir,ch,adds,dels,t)| json!({
                        "dir": dir, "churn": ch, "adds": adds, "dels": dels, "touches": t
                    })).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            let mut t = Table::new();
            t.load_preset(UTF8_HORIZONTAL_ONLY).set_header(vec![
                "Directory",
                "Churn",
                "Adds",
                "Dels",
                "Touches",
            ]);

            for (dir, churn, adds, dels, touches) in dir_rows.into_iter().take(self.limit) {
                t.add_row(vec![
                    dir,
                    format!("{:.1}", churn),
                    adds.to_string(),
                    dels.to_string(),
                    touches.to_string(),
                ]);
            }

            println!(
                "♨️  Churn (last {} days) — by directory (depth {})",
                self.window_days, self.depth
            );
            println!("{t}");
            return Ok(());
        }

        // by == "file"
        if json {
            let payload = json!({
                "by": "file",
                "window_days": self.window_days,
                "rows": entries.iter().take(self.limit).map(|e| json!({
                    "file": e.path, "churn": e.churn, "adds": e.adds, "dels": e.dels, "touches": e.touches
                })).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        let mut t = Table::new();
        t.load_preset(UTF8_HORIZONTAL_ONLY)
            .set_header(vec!["File", "Churn", "Adds", "Dels", "Touches"]);

        for e in entries.drain(..).take(self.limit) {
            t.add_row(vec![
                e.path,
                format!("{:.1}", e.churn),
                e.adds.to_string(),
                e.dels.to_string(),
                e.touches.to_string(),
            ]);
        }

        println!("♨️  Churn (last {} days) — by file", self.window_days);

        println!("{t}");
        Ok(())
    }
}
