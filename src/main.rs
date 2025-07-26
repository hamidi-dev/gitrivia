mod analysis;
mod cli;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Stats { path, limit, sort_desc } => {
            analysis::print_stats(&path, limit, sort_desc);
        }

        Commands::TopAuthors { path, since } => {
            analysis::top_authors(&path, since);
        }

        Commands::AuthorActivity { path, author } => {
            analysis::author_activity(&path, &author);
        }

        Commands::BlameSummary { path, file, json } => {
            analysis::blame_summary(&path, &file, json);
        }

        Commands::FileContributions { path, json } => {
            analysis::file_contributions(&path, json);
        }

        Commands::CommitTimes { path, json } => {
            analysis::commit_times(&path, json);
        }

        Commands::FirstCommits { path, json } => {
            analysis::first_commits(&path, json);
        }

        Commands::TopCoauthors { path, json } => {
            analysis::top_coauthors(&path, json);
        }

        Commands::BusFactor { path, json, threshold } => {
            analysis::bus_factor(&path, json, threshold);
        }
    }
}

