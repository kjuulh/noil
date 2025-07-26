use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::format;

#[derive(Parser)]
pub struct FmtCommand {}

impl FmtCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let mut stdin = tokio::io::stdin();
        let mut buffer = Vec::new();

        stdin.read_to_end(&mut buffer).await?;

        let input = String::from_utf8_lossy(&buffer);

        let output = format::format(&input)?;

        let mut stdout = tokio::io::stdout();
        stdout.write_all(output.as_bytes()).await?;
        stdout.flush().await?;

        Ok(())
    }
}
