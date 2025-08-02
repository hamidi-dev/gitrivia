use anyhow::Result;
use clap::Args;
use serde_json::json;
use crate::{domain::{git::RepoExt, stats as d}, utils::fmt_date};
use crate::commands::Global;

#[derive(Debug, Args)]
pub struct AuthorActivity {
    #[arg(short, long, default_value=".")]
    pub path: String,
    #[arg(short, long)]
    pub author: String,
}

impl super::Runnable for AuthorActivity {
    fn run(self, g: &Global) -> Result<()> {
        let repo  = RepoExt::open(&self.path)?;
        let stats = d::collect_commits(repo.repo(), usize::MAX, None);

        match stats.data.get(&self.author) {
            Some(m) => {
                if g.json {
                    let payload = json!({
                        "author": self.author,
                        "count": m.count,
                        "first": fmt_date(m.first),
                        "last":  fmt_date(m.last),
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    println!("{:<30} {:>4} commits 🗓  {} → {}",
                        self.author, m.count, fmt_date(m.first), fmt_date(m.last));
                }
            }
            None => {
                if g.json {
                    let payload = json!({ "author": self.author, "found": false });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    eprintln!("No commits by {}", self.author);
                }
            }
        }
        Ok(())
    }
}

