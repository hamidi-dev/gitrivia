use crate::commands::Global;
use crate::domain::{coauthors, git::RepoExt};
use anyhow::Result;
use clap::Args;

/// Identify author pairs that frequently touch the same files.
///
/// Helps uncover collaboration clusters or potential review buddies.
#[derive(Debug, Args)]
pub struct TopCoauthors {
    /// Path to the Git repository.
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Emit JSON even when the global flag is not set.
    #[arg(long)]
    pub json: bool,
}

impl super::Runnable for TopCoauthors {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let pairs = coauthors::top_coauthors(repo.repo())?;
        if g.json || self.json {
            println!("{}", serde_json::to_string_pretty(&pairs)?);
        } else {
            for (pair, count) in pairs {
                println!("{:<50} {:>3} shared files", pair, count);
            }
        }
        Ok(())
    }
}
