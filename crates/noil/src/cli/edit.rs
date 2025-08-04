use std::{
    env::temp_dir,
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

use ansi_term::Color;
use anyhow::{Context, bail};
use clap::Parser;
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};

use crate::{
    commit::{Action, print_changes},
    encode_rand,
    models::Operation,
    output::get_outputs,
    parse,
};

const PREVIEW: bool = false;

#[derive(Parser)]
pub struct EditCommand {
    #[arg()]
    path: PathBuf,

    #[arg(long = "chooser-file", env = "NOIL_CHOOSER_FILE")]
    chooser_file: Option<PathBuf>,

    #[arg(long = "commit")]
    commit: bool,

    #[arg(long = "quiet")]
    quiet: bool,
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

        let path = &self.get_path().await.context("get path")?;
        let output = get_outputs(path, true)
            .await
            .context(format!("get output: {}", path.display()))?;
        file.write_all(output.as_bytes())
            .await
            .context("write contents for edit")?;
        file.flush().await.context("flush contents for edit")?;

        let editor = std::env::var("EDITOR").context("EDITOR not found in env")?;

        loop {
            let mut cmd = tokio::process::Command::new(editor.trim());
            cmd.arg(&file_path);

            if !std::io::stdout().is_terminal() {
                let tty = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open("/dev/tty")
                    .await
                    .context("open tty")?;
                let tty_in = tty.try_clone().await.context("clone ttyin")?;
                let tty_out = tty.try_clone().await.context("clone ttyout")?;

                cmd.stdin(Stdio::from(tty_in.into_std().await))
                    .stdout(Stdio::from(tty_out.into_std().await))
                    .stderr(Stdio::from(tty.into_std().await));
            }

            let mut process = cmd.spawn().context("command not found")?;
            let status = process.wait().await.context("editor closed prematurely")?;
            if !status.success() {
                let code = status.code().unwrap_or(-1);
                anyhow::bail!("editor exited: {code}");
            }

            let noil_content = tokio::fs::read_to_string(&file_path)
                .await
                .context("read noil file")?;

            let res = if !self.commit {
                print_changes(&noil_content, PREVIEW).await
            } else {
                Ok(Action::Apply {
                    original: noil_content,
                })
            };

            let action = match res {
                Ok(a) => a,
                Err(e) => {
                    eprintln!(
                        "Invalid operation\n{}\n\nreverting to edit on any key press: ",
                        Color::Red.normal().paint(format!("{e:?}"))
                    );

                    wait_user().await.context("user finished prematurely")?;

                    continue;
                }
            };

            match action {
                Action::Quit => return Ok(()),
                Action::Apply { original } => {
                    return apply(
                        &original,
                        ApplyOptions {
                            chooser_file: self.chooser_file.clone(),
                            quiet: self.quiet,
                        },
                    )
                    .await;
                }
                Action::Edit => continue,
            }
        }
    }

    async fn get_path(&self) -> anyhow::Result<PathBuf> {
        let path_str = self.path.display().to_string();
        let expanded_path = shellexpand::full(&path_str)?;
        let path = PathBuf::from(expanded_path.to_string());

        if !path.exists() {
            anyhow::bail!("path: {} does not exist", self.path.display());
        }

        if path.is_file() {
            let parent_path = path
                .parent()
                .map(|p| p.to_path_buf())
                .ok_or(anyhow::anyhow!("parent doesn't exist for file"))?;

            if parent_path.display().to_string() == "" {
                return Ok(PathBuf::from("."));
            }

            return Ok(parent_path);
        }

        Ok(path.clone())
    }
}

async fn wait_user() -> Result<(), anyhow::Error> {
    let mut stderr = std::io::stderr();
    stderr.flush()?;
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input_buf = String::new();
    reader
        .read_line(&mut input_buf)
        .await
        .context("failed to read stdin")?;
    Ok(())
}

#[derive(Default, Clone, Debug)]
pub struct ApplyOptions {
    pub chooser_file: Option<PathBuf>,
    pub quiet: bool,
}

/// the philosphy behind apply is that we try unlike normal file system operations to be idempotent.
/// This is mainly for 2 reasons.
///
/// 1. A lot of operations are processed, stopping in the middle because of an error would ruing your previous procedure that you now have to go back and fix
/// 2. A .noil recipe can be rerun, having small issues disrupt the work would be counterproductive, as the .noil language is not powerful enough to handle the flexibility required for file checking
///
/// All in all apply is mostly idempotent, and won't override files, it tries to be as non destructive as possible. For example move will only throw a warning if the source file doesn't exists, but the destination does
pub async fn apply(input: &str, options: ApplyOptions) -> anyhow::Result<()> {
    if !options.quiet {
        eprintln!("applying changes");
    }

    let noil_index = parse::parse_input(input).context("parse input")?;

    let mut open_files = Vec::new();

    for file in &noil_index.files {
        let path = &file.path;
        match &file.entry.operation {
            Operation::Existing { .. } => {
                // Noop
            }
            Operation::Open { .. } => {
                if path.to_string_lossy().ends_with("/") {
                    // We can't open directories, so they're skipped
                    continue;
                }

                open_files.push(path);
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
                    continue;
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

    if let Some(chooser_file) = &options.chooser_file {
        tracing::debug!("creating chooser file");
        if let Some(parent) = chooser_file.parent()
            && !chooser_file.exists()
        {
            tokio::fs::create_dir_all(parent)
                .await
                .context("parent dir for chooser file")?;
        }

        let mut file = tokio::fs::File::create(chooser_file)
            .await
            .context("create new chooser file")?;

        let open_files = open_files
            .iter()
            .map(|i| i.display().to_string())
            .collect::<Vec<_>>();

        file.write_all(open_files.join("\n").as_bytes())
            .await
            .context("write chosen files")?;
        file.flush().await.context("flush chosen file")?;
    }

    Ok(())
}

async fn copy(source: &Path, dest: &Path) -> anyhow::Result<()> {
    let mut paths = Vec::new();

    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;

        tracing::debug!("copying path: {}", entry.path().display());

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
        tracing::info!("copying dir: {}", dest.display());
        tokio::fs::create_dir_all(&dest).await.context("copy dir")?;
    }

    if src.is_file() {
        tracing::info!("copying file: {}", dest.display());
        tokio::fs::copy(&src, &dest).await.context("copy file")?;
    }

    Ok(())
}
