use crate::commands::Global;
use crate::domain::{git::RepoExt, times};
use anyhow::Result;
use clap::Args;

/// Aggregate commit counts into hourly buckets for each author.
///
/// Reveals work patterns or time‑zone differences within the team.
#[derive(Debug, Args)]
pub struct CommitTimes {
    /// Path to the Git repository to analyse.
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Emit JSON even when the global flag is not set.
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
