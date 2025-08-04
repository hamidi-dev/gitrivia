use crate::commands::Global;
use crate::{
    domain::{firsts, git::RepoExt},
    utils::fmt_date,
};
use anyhow::Result;
use clap::Args;

/// List when each contributor made their first commit.
///
/// Helps gauge longevity of contributors or find long‑term maintainers.
#[derive(Debug, Args)]
pub struct FirstCommits {
    /// Path to the Git repository to analyse.
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Emit JSON even when the global flag is not set.
    #[arg(long)]
    pub json: bool,
}

impl super::Runnable for FirstCommits {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let map = firsts::first_commits(repo.repo())?;
        if g.json || self.json {
            let as_str = map
                .into_iter()
                .map(|(k, v)| (k, fmt_date(v)))
                .collect::<std::collections::BTreeMap<_, _>>();
            println!("{}", serde_json::to_string_pretty(&as_str)?);
        } else {
            for (email, dt) in map {
                println!("{:<30} {}", email, fmt_date(dt));
            }
        }
        Ok(())
    }
}
