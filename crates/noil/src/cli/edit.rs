use std::{
    env::temp_dir,
    io::Write,
    path::{Path, PathBuf},
};

use ansi_term::Color;
use anyhow::{Context, bail};
use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{
    commit::{Action, print_changes},
    encode_rand,
    models::Operation,
    output::get_outputs,
    parse,
};

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

        loop {
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

            let res = print_changes(&noil_content).await;

            let action = match res {
                Ok(a) => a,
                Err(e) => {
                    eprintln!(
                        "Invalid operation\n{}\n\nreverting to edit on any key press: ",
                        Color::Red.normal().paint(format!("{e:?}"))
                    );

                    wait_user().await?;

                    continue;
                }
            };

            match action {
                Action::Quit => return Ok(()),
                Action::Apply { original } => {
                    return apply(&original).await;
                }
                Action::Edit => continue,
            }
        }
    }
}

async fn wait_user() -> Result<(), anyhow::Error> {
    let mut stderr = std::io::stderr();
    stderr.flush()?;
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input_buf = String::new();
    reader.read_line(&mut input_buf).await?;
    Ok(())
}

/// the philosphy behind apply is that we try unlike normal file system operations to be idempotent.
/// This is mainly for 2 reasons.
///
/// 1. A lot of operations are processed, stopping in the middle because of an error would ruing your previous procedure that you now have to go back and fix
/// 2. A .noil recipe can be rerun, having small issues disrupt the work would be counterproductive, as the .noil language is not powerful enough to handle the flexibility required for file checking
///
/// All in all apply is mostly idempotent, and won't override files, it tries to be as non destructive as possible. For example move will only throw a warning if the source file doesn't exists, but the destination does
pub async fn apply(input: &str) -> anyhow::Result<()> {
    eprintln!("applying changes");

    let noil_index = parse::parse_input(input).context("parse input")?;

    for file in &noil_index.files {
        let path = &file.path;
        match &file.entry.operation {
            Operation::Existing { .. } => {
                // Noop
            }
            Operation::Add => {
                tracing::debug!("creating file");

                if path.exists() {
                    tracing::warn!("path already exists");
                    continue;
                }

                // is dir
                if path.to_string_lossy().ends_with("/") {
                    tokio::fs::create_dir_all(&path)
                        .await
                        .context("add directory")?;
                    tracing::info!("added directory");
                    continue;
                }

                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(&parent)
                        .await
                        .context("create parent dir for add file")?;
                }

                tokio::fs::File::create(&path).await.context("add file")?;

                tracing::info!("added file");
            }
            Operation::Copy { index } => {
                tracing::debug!("copying file");

                let existing = noil_index.get_existing(index).ok_or(anyhow::anyhow!(
                    "entry with index: '{}' does not exist for copy",
                    index
                ))?;
                if !existing.path.exists() {
                    bail!("existing does not exist for copy")
                }

                if path.exists() {
                    tracing::warn!("path already exists, cannot copy");
                    continue;
                }

                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(&parent)
                        .await
                        .context("create parent dir for copy")?;
                }

                if existing.path.is_dir() {
                    tracing::debug!("copying dir");
                    copy(&existing.path, path).await?;
                }

                tokio::fs::copy(&existing.path, &path)
                    .await
                    .context("copy file for copy")?;
            }
            Operation::Delete { .. } => {
                tracing::debug!("deleting file");

                if !path.exists() {
                    tracing::warn!("path doesn't exist");
                    continue;
                }

                if path.is_dir() {
                    tokio::fs::remove_dir_all(&path)
                        .await
                        .context("remove path for delete")?;
                    continue;
                }

                tokio::fs::remove_file(&path)
                    .await
                    .context("remove file for delete")?
            }
            Operation::Move { index } => {
                tracing::debug!("moving file");

                let existing = noil_index.get_existing(index);

                if existing.is_none() {
                    // If the destination exists, but the existing one doesn't we assume it has already been moved
                    if path.exists() {
                        tracing::warn!("destination file looks to already have been moved");
                        continue;
                    }

                    anyhow::bail!("neither existing, or destination exists for move");
                }
                let existing = existing.unwrap();

                if path.exists() {
                    anyhow::bail!("destination already exists cannot move");
                }

                tokio::fs::rename(&existing.path, path)
                    .await
                    .context("move path")?;
            }
        }
    }

    Ok(())
}

async fn copy(source: &Path, dest: &Path) -> anyhow::Result<()> {
    let mut paths = Vec::new();

    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        paths.push(entry.path().strip_prefix(source)?.to_path_buf());
    }

    for path in paths {
        let source = source.join(&path);
        let dest = dest.join(&path);

        copy_path(&source, &dest).await.context(anyhow::anyhow!(
            "copy path: (src: {}, dest: {})",
            source.display(),
            dest.display()
        ))?;
    }

    Ok(())
}

async fn copy_path(src: &Path, dest: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("copy dir create parent dir")?;
    }

    if src.is_dir() {
        tokio::fs::create_dir_all(&dest).await.context("copy dir")?;
    }

    if dest.is_file() {
        tokio::fs::copy(&src, &dest).await.context("copy file")?;
    }

    Ok(())
}
