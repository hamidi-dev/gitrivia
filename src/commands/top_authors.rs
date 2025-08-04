use crate::domain::{git::RepoExt, stats as d};
use crate::{commands::Global, utils::fmt_date};
use anyhow::Result;
use chrono::{Local, NaiveDate, TimeZone};
use clap::Args;
use serde_json::json;

/// Rank authors by number of commits.
///
/// Useful for identifying the most active contributors over the entire
/// history or since a specific date.
#[derive(Debug, Args)]
pub struct TopAuthors {
    /// Path to the Git repository to analyse.
    /// Defaults to the current directory.
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Only include commits on or after this date (YYYY-MM-DD).
    /// If omitted, the full commit history is considered.
    #[arg(short, long)]
    pub since: Option<NaiveDate>,
}

impl super::Runnable for TopAuthors {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let since_dt = self.since.map(|d| {
            Local
                .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
                .unwrap()
        });
        let stats = d::collect_commits(repo.repo(), usize::MAX, since_dt);

        if g.json {
            let authors: Vec<_> = stats
                .data
                .iter()
                .map(|(email, m)| {
                    json!({
                        "email": email,
                        "count": m.count,
                        "first": fmt_date(m.first),
                        "last":  fmt_date(m.last),
                    })
                })
                .collect();
            let payload = json!({
                "since": self.since.map(|d| d.to_string()),
                "authors_sorted_desc": g.desc,
                "authors": authors
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("Authors since {:?}:", self.since);
            for line in stats.formatted_lines(g.desc) {
                println!("{line}");
            }
        }
        Ok(())
    }
}
