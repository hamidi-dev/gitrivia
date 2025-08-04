use crate::commands;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "gitrivia",
    version,
    about = "✨ Explore who did what, when — in any Git repo",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Emit machine‑readable JSON instead of tables.
    ///
    /// This flag can be used with any subcommand and mirrors the
    /// `--json` option many subcommands offer individually.
    #[arg(long, global = true)]
    pub json: bool,

    /// Sort results in descending order by default.
    ///
    /// Some commands allow per‑call sorting options; this flag acts as a
    /// convenient global override.
    #[arg(long, global = true)]
    pub desc: bool,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// 📊 Overall commit stats per author
    ///
    /// Provides a health snapshot of the repository and highlights the
    /// most active contributors.
    Stats(commands::Stats),

    /// 🏆 Most prolific authors since a given date
    ///
    /// Counts commits per author, optionally starting from a specific
    /// date, to see who is currently most active.
    TopAuthors(commands::TopAuthors),

    /// 🧑‍💻 Show first and last commit dates for one author
    ///
    /// Useful for onboarding checks or when verifying an author's
    /// activity span in the repository.
    AuthorActivity(commands::AuthorActivity),

    /// 👀 Who wrote which lines of a file
    ///
    /// Summarises line ownership for a file using `git blame` data.
    BlameSummary(commands::BlameSummary),

    /// 📁 Per-author commit heatmap by file
    ///
    /// Shows which authors have touched which files and how often,
    /// helping with code review routing.
    FileContributions(commands::FileContributions),

    /// 🕒 Commit time-of-day distribution
    ///
    /// Breaks down commits into 24 time buckets per author to reveal
    /// work patterns across the team.
    CommitTimes(commands::CommitTimes),

    /// 🥇 First commit by each author
    ///
    /// Outputs when each contributor first appeared in the commit
    /// history of the project.
    FirstCommits(commands::FirstCommits),

    /// 🤝 Top co-authors (shared files)
    ///
    /// Finds pairs of authors who frequently change the same files,
    /// indicating areas of collaboration.
    TopCoauthors(commands::TopCoauthors),

    /// 🚍 Bus factor warnings (file or directory)
    ///
    /// Detects files or directories dominated by a single author,
    /// signalling knowledge silos.
    BusFactor(commands::BusFactor),

    /// ♨️ Churn (recent file/directory volatility)
    ///
    /// Ranks paths by recent change activity to highlight unstable or
    /// frequently modified areas.
    Churn(commands::Churn),
}
