use tokio::io::AsyncReadExt;

use crate::{
    cli::edit::apply,
    commit::{Action, print_changes},
};

#[derive(clap::Parser)]
pub struct ApplyCommand {
    #[arg(long = "commit")]
    commit: bool,
}

impl ApplyCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let mut stdin = tokio::io::stdin();
        let mut buffer = Vec::new();

        stdin.read_to_end(&mut buffer).await?;

        let input = String::from_utf8_lossy(&buffer);

        if !self.commit {
            let action = print_changes(&input, !self.commit).await?;
            match action {
                Action::Quit => Ok(()),
                Action::Apply { original } => apply(&original).await,
                Action::Edit => todo!(),
            }
        } else {
            apply(&input).await
        }
    }
}
