use anyhow::Result;
use clap::Args;
use crate::{domain::{git::RepoExt, firsts}, utils::fmt_date};
use crate::commands::Global;

#[derive(Debug, Args)]
pub struct FirstCommits {
    #[arg(short, long, default_value=".")]
    pub path: String,
    #[arg(long)]
    pub json: bool,
}

impl super::Runnable for FirstCommits {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let map  = firsts::first_commits(repo.repo())?;
        if g.json || self.json {
            let as_str = map.into_iter()
                .map(|(k,v)| (k, fmt_date(v)))
                .collect::<std::collections::BTreeMap<_,_>>();
            println!("{}", serde_json::to_string_pretty(&as_str)?);
        } else {
            for (email, dt) in map {
                println!("{:<30} {}", email, fmt_date(dt));
            }
        }
        Ok(())
    }
}

