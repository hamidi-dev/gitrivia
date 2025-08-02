use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub struct Global {
    /// Global JSON output toggle (overrides per-command flags)
    pub json: bool,
    /// Global default for “descending” ordering in commands that support it
    pub desc: bool,
}

pub trait Runnable {
    fn run(self, g: &Global) -> Result<()>;
}

pub mod stats;
pub mod top_authors;
pub mod author_activity;
pub mod blame_summary;
pub mod file_contributions;
pub mod commit_times;
pub mod first_commits;
pub mod top_coauthors;
pub mod bus_factor;
pub mod churn;

pub use stats::Stats;
pub use top_authors::TopAuthors;
pub use author_activity::AuthorActivity;
pub use blame_summary::BlameSummary;
pub use file_contributions::FileContributions;
pub use commit_times::CommitTimes;
pub use first_commits::FirstCommits;
pub use top_coauthors::TopCoauthors;
pub use bus_factor::BusFactor;
pub use churn::Churn;
