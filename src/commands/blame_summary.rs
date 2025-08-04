use crate::commands::Global;
use crate::domain::{blame, git::RepoExt};
use anyhow::Result;
use clap::Args;
use std::path::Path;

/// Summarise line ownership for a single file via `git blame`.
///
/// The command reports how many lines each author owns and can emit JSON
/// for tooling or a plain table for quick inspection.
#[derive(Debug, Args)]
pub struct BlameSummary {
    /// File to analyse relative to the repository root.
    #[arg(short, long)]
    pub file: String,

    /// Path to the Git repository containing the file.
    /// Defaults to the current directory.
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Output JSON regardless of the global `--json` flag.
    #[arg(long)]
    pub json: bool,
}

impl super::Runnable for BlameSummary {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let counts = blame::blame_counts(repo.repo(), Path::new(&self.file))?;
        if g.json || self.json {
            println!("{}", serde_json::to_string_pretty(&counts)?);
        } else {
            for (email, count) in counts {
                println!("{:<30} {:>4} lines", email, count);
            }
        }
        Ok(())
    }
}
