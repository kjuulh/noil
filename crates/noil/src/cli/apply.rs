use tokio::io::AsyncReadExt;

use crate::commit::write_changes;

#[derive(clap::Parser)]
pub struct ApplyCommand {}

impl ApplyCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let mut stdin = tokio::io::stdin();
        let mut buffer = Vec::new();

        stdin.read_to_end(&mut buffer).await?;

        let input = String::from_utf8_lossy(&buffer);

        write_changes(&input).await?;

        Ok(())
    }
}
