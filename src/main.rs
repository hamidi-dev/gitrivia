mod analysis;
mod cli;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Stats {
            path,
            limit,
            sort_desc,
        } => {
            analysis::print_stats(&path, limit, sort_desc);
        }
        Commands::TopAuthors { path, since } => {
            analysis::top_authors(&path, since);
        }
        Commands::AuthorActivity { path, author } => {
            analysis::author_activity(&path, &author);
        }
        Commands::BlameSummary { file, json } => {
            analysis::blame_summary(&file, json);
        }
        Commands::FileContributions { json } => {
            analysis::file_contributions(".", json);
        }
        Commands::CommitTimes { json } => {
            analysis::commit_times(".", json);
        }
        Commands::FirstCommits { json } => {
            analysis::first_commits(".", json);
        }
        Commands::TopCoauthors { json } => {
            analysis::top_coauthors(".", json);
        }
        Commands::BusFactor { json, threshold } => {
            analysis::bus_factor(".", json, threshold);
        }
    }
}
