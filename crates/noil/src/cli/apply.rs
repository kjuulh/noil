use std::path::PathBuf;

use tokio::io::AsyncReadExt;

use crate::{
    cli::edit::{ApplyOptions, apply},
    commit::{Action, print_changes},
};

#[derive(clap::Parser)]
pub struct ApplyCommand {
    #[arg(long = "commit")]
    commit: bool,

    #[arg(long = "chooser-file", env = "NOIL_CHOOSER_FILE")]
    chooser_file: Option<PathBuf>,
}

impl ApplyCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let mut stdin = tokio::io::stdin();
        let mut buffer = Vec::new();

        stdin.read_to_end(&mut buffer).await?;

        let input = String::from_utf8_lossy(&buffer);

        if !self.commit {
            let action = print_changes(&input, !self.commit).await?;
            let res = match action {
                Action::Quit => Ok(()),
                Action::Apply { original } => {
                    apply(
                        &original,
                        ApplyOptions {
                            chooser_file: self.chooser_file.clone(),
                        },
                    )
                    .await
                }
                Action::Edit => todo!(),
            };

            eprintln!("\nin preview mode: add (--commit) to perform actions");

            res
        } else {
            apply(
                &input,
                ApplyOptions {
                    chooser_file: self.chooser_file.clone(),
                },
            )
            .await
        }
    }
}
