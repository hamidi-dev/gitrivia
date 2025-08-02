use anyhow::Result;
use clap::Args;
use crate::domain::{git::RepoExt, bus_factor};
use crate::commands::Global;

#[derive(Debug, Args)]
pub struct BusFactor {
    #[arg(short, long, default_value=".")]
    pub path: String,
    #[arg(long)]
    pub json: bool,
    /// Ownership % threshold (0.0..1.0), default 0.75
    #[arg(long, default_value="0.75")]
    pub threshold: f64,
}

impl super::Runnable for BusFactor {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let map  = bus_factor::bus_factor(&self.path, repo.repo(), self.threshold)?;
        if g.json || self.json {
            println!("{}", bus_factor::as_busfactor_json(&map));
        } else {
            for (file, (author, ratio)) in map {
                println!("⚠️  {:<30} {:>5.1}% by {}", file, ratio * 100.0, author);
            }
        }
        Ok(())
    }
}

