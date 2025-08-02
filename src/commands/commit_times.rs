use anyhow::Result;
use clap::Args;
use crate::domain::{git::RepoExt, times};
use crate::commands::Global;

#[derive(Debug, Args)]
pub struct CommitTimes {
    #[arg(short, long, default_value=".")]
    pub path: String,
    #[arg(long)]
    pub json: bool,
}

impl super::Runnable for CommitTimes {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let map = times::commit_times(repo.repo())?;
        if g.json || self.json {
            println!("{}", serde_json::to_string_pretty(&map)?);
        } else {
            for (email, buckets) in map {
                println!("{email}");
                for (label, count) in buckets {
                    println!("  {:<10} {}", label, count);
                }
            }
        }
        Ok(())
    }
}

