use clap::{Parser, Subcommand};
use crate::commands;

#[derive(Parser)]
#[command(
    name = "gitrivia",
    version,
    about = "✨ Explore who did what, when — in any Git repo",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Global JSON output (works with any subcommand)
    #[arg(long, global = true)]
    pub json: bool,

    /// Global default for descending sort where applicable
    #[arg(long, global = true)]
    pub desc: bool,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// 📊 Overall commit stats per author
    Stats(commands::Stats),

    /// 🏆 Most prolific authors since a given date
    TopAuthors(commands::TopAuthors),

    /// 🧑‍💻 Show first and last commit dates for one author
    AuthorActivity(commands::AuthorActivity),

    /// 👀 Who wrote which lines of a file
    BlameSummary(commands::BlameSummary),

    /// 📁 Per-author commit heatmap by file
    FileContributions(commands::FileContributions),

    /// 🕒 Commit time-of-day distribution
    CommitTimes(commands::CommitTimes),

    /// 🥇 First commit by each author
    FirstCommits(commands::FirstCommits),

    /// 🤝 Top co-authors (shared files)
    TopCoauthors(commands::TopCoauthors),

    /// 🚍 Bus factor warnings (file or directory)
    BusFactor(commands::BusFactor),

    /// ♨️ Churn (recent file/directory volatility)
    Churn(commands::Churn),
}

