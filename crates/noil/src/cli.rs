use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::cli::{apply::ApplyCommand, edit::EditCommand, fmt::FmtCommand, output::OutputCommand};

mod apply;
mod edit;
mod fmt;
mod output;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Command {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg()]
    path: Option<PathBuf>,

    #[arg(long = "no-color", default_value = "false")]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    Edit(EditCommand),
    Apply(ApplyCommand),
    Fmt(FmtCommand),
}

pub async fn execute() -> anyhow::Result<()> {
    let cli = Command::parse();
    tracing::debug!("Starting cli");

    match cli.command {
        Some(Commands::Edit(cmd)) => cmd.execute().await,
        Some(Commands::Fmt(cmd)) => cmd.execute().await,
        Some(Commands::Apply(cmd)) => cmd.execute().await,
        None => {
            let path = match &cli.path {
                Some(path) => path,
                None => anyhow::bail!("a path is required if just using noil"),
            };

            OutputCommand {}.execute(path, cli.no_color).await
        }
    }
}
