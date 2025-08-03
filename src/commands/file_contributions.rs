use crate::commands::Global;
use crate::domain::{files, git::RepoExt};
use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct FileContributions {
    #[arg(short, long, default_value = ".")]
    pub path: String,
    #[arg(long)]
    pub json: bool,
}

impl super::Runnable for FileContributions {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let map = files::file_contributions(repo.repo())?;
        if g.json || self.json {
            println!("{}", serde_json::to_string_pretty(&map)?);
        } else {
            for (file, authors) in map {
                println!("{file}");
                for (author, count) in authors {
                    println!("  {:<30} {} commits", author, count);
                }
            }
        }
        Ok(())
    }
}
