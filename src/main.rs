mod cli;
mod commands;
mod domain;
mod presentation;
mod utils;

use clap::Parser;
use commands::{Global, Runnable};

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let g = Global {
        json: cli.json,
        desc: cli.desc,
    };

    match cli.command {
        cli::CliCommand::Stats(c) => c.run(&g),
        cli::CliCommand::TopAuthors(c) => c.run(&g),
        cli::CliCommand::AuthorActivity(c) => c.run(&g),
        cli::CliCommand::BlameSummary(c) => c.run(&g),
        cli::CliCommand::FileContributions(c) => c.run(&g),
        cli::CliCommand::CommitTimes(c) => c.run(&g),
        cli::CliCommand::FirstCommits(c) => c.run(&g),
        cli::CliCommand::TopCoauthors(c) => c.run(&g),
        cli::CliCommand::BusFactor(c) => c.run(&g),
        cli::CliCommand::Churn(c) => c.run(&g),
    }
}
