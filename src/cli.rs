use chrono::NaiveDate;
use clap::{Parser, Subcommand};

/// 🔍 Git repository activity explorer
#[derive(Parser)]
#[command(
    name = "gitrivia",
    version,
    about = "✨ Explore who did what, when — in any Git repo",
    long_about = None,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 📊 Overall commit stats per author
    Stats {
        /// 📁 Path to the Git repo
        #[arg(short, long, default_value = ".")]
        path: String,

        /// 🔢 Max number of commits to inspect (default: unlimited)
        #[arg(short, long)]
        limit: Option<usize>,

        /// 🔽 Sort descending by number of commits
        #[arg(long)]
        sort_desc: bool,
    },

    /// 🏆 Most prolific authors since a given date
    TopAuthors {
        /// 📁 Path to the Git repo
        #[arg(short, long, default_value = ".")]
        path: String,

        /// 🗓 Only include commits on or after this date (YYYY-MM-DD)
        #[arg(short, long)]
        since: Option<NaiveDate>,
    },

    /// 🧑‍💻 Show first and last commit dates for one author
    AuthorActivity {
        /// 📁 Path to the Git repo
        #[arg(short, long, default_value = ".")]
        path: String,

        /// ✉️ Author email (exact match)
        #[arg(short, long)]
        author: String,
    },
    /// 👀 Who wrote which lines of a file
    BlameSummary {
        #[arg(short, long)]
        file: String,

        #[arg(long)]
        json: bool,
    },

    /// 📁 Per-author commit heatmap by file
    FileContributions {
        #[arg(long)]
        json: bool,
    },

    /// 🕒 Commit time-of-day distribution
    CommitTimes {
        #[arg(long)]
        json: bool,
    },

    /// 🥇 First commit by each author
    FirstCommits {
        #[arg(long)]
        json: bool,
    },

    /// 🤝 Top co-authors (shared files)
    TopCoauthors {
        #[arg(long)]
        json: bool,
    },

    /// 🚍 Bus factor warnings
    BusFactor {
        #[arg(long)]
        json: bool,

        /// 🚨 Ownership % threshold for warning (default: 0.75)
        #[arg(long, default_value = "0.75")]
        threshold: f64,
    },
}
