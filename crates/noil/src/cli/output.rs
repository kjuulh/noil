use std::path::Path;

use clap::Parser;
use tokio::io::AsyncWriteExt;

use crate::output::get_outputs;

#[derive(Parser)]
pub struct OutputCommand {}

impl OutputCommand {
    pub async fn execute(&self, path: &Path, no_color: bool) -> anyhow::Result<()> {
        let output = get_outputs(path, no_color).await?;

        let mut stdout = tokio::io::stdout();
        stdout.write_all(output.as_bytes()).await?;
        stdout.flush().await?;

        Ok(())
    }
}
