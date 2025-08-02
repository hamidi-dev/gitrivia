use anyhow::Result;
use clap::Args;
use crate::domain::{git::RepoExt, coauthors};
use crate::commands::Global;

#[derive(Debug, Args)]
pub struct TopCoauthors {
    #[arg(short, long, default_value=".")]
    pub path: String,
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

