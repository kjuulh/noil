use std::{env::temp_dir, path::PathBuf};

use anyhow::Context;
use clap::Parser;
use tokio::io::AsyncWriteExt;

use crate::{commit::write_changes, encode_rand, output::get_outputs};

#[derive(Parser)]
pub struct EditCommand {
    #[arg()]
    path: PathBuf,
}

impl EditCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let mut small_id = Vec::with_capacity(8);
        for id in small_id.iter_mut() {
            *id = encode_rand::ALPHABET
                [rand::random_range(0..(encode_rand::ALPHABET_LEN as u8)) as usize];
        }
        let small_id = String::from_utf8_lossy(&small_id);

        let file_path = temp_dir()
            .join("noil")
            .join(small_id.to_string())
            .join("buf.noil");

        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("failed to create temp dir file")?;
        }

        let mut file = tokio::fs::File::create(&file_path)
            .await
            .context("create temp file for noil")?;

        let output = get_outputs(&self.path, true).await?;
        file.write_all(output.as_bytes()).await?;
        file.flush().await?;

        let editor = std::env::var("EDITOR").context("EDITOR not found in env")?;

        let mut cmd = tokio::process::Command::new(editor.trim());
        cmd.arg(&file_path);

        let mut process = cmd.spawn()?;
        let status = process.wait().await.context("editor closed prematurely")?;
        if !status.success() {
            let code = status.code().unwrap_or(-1);
            anyhow::bail!("editor exited: {code}");
        }

        let noil_content = tokio::fs::read_to_string(&file_path)
            .await
            .context("read noil file")?;

        write_changes(&noil_content).await?;
        Ok(())
    }
}
