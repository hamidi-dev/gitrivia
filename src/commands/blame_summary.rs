use anyhow::Result;
use clap::Args;
use std::path::Path;
use crate::domain::{git::RepoExt, blame};
use crate::commands::Global;

#[derive(Debug, Args)]
pub struct BlameSummary {
    #[arg(short, long)]
    pub file: String,
    #[arg(short, long, default_value=".")]
    pub path: String,
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
            for (email, count) in counts { println!("{:<30} {:>4} lines", email, count); }
        }
        Ok(())
    }
}

