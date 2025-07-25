use std::{
    env::temp_dir,
    fmt::{Display, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
    Edit {
        #[arg()]
        path: PathBuf,
    },
    Apply {},
    Fmt {},
}

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const ALPHABET_LEN: u32 = ALPHABET.len() as u32;

fn encode_256bit_base36(input: &[u8; 32]) -> String {
    let mut num = *input;
    let mut output = Vec::with_capacity(52); // log_36(2^256) ≈ 50.7

    while num.iter().any(|&b| b != 0) {
        let mut rem: u32 = 0;
        for byte in num.iter_mut() {
            let acc = ((rem as u16) << 8) | *byte as u16;
            *byte = (acc / ALPHABET_LEN as u16) as u8;
            rem = (acc % ALPHABET_LEN as u16) as u32;
        }
        output.push(ALPHABET[rem as usize]);
    }

    if output.is_empty() {
        output.push(ALPHABET[0]);
    }

    output.reverse();
    String::from_utf8(output).unwrap()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Command::parse();
    tracing::debug!("Starting cli");

    match cli.command {
        Some(Commands::Edit { path }) => {
            let mut small_id = Vec::with_capacity(8);
            for id in small_id.iter_mut() {
                *id = ALPHABET[rand::random_range(0..(ALPHABET_LEN as u8)) as usize];
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

            let output = get_outputs(&path, true).await?;
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

            //todo!()
        }
        Some(Commands::Fmt {}) => {
            let mut stdin = tokio::io::stdin();
            let mut buffer = Vec::new();

            stdin.read_to_end(&mut buffer).await?;

            let input = String::from_utf8_lossy(&buffer);

            let output = format(&input)?;

            let mut stdout = tokio::io::stdout();
            stdout.write_all(output.as_bytes()).await?;
            stdout.flush().await?;
        }
        Some(Commands::Apply {}) => {
            let mut stdin = tokio::io::stdin();
            let mut buffer = Vec::new();

            stdin.read_to_end(&mut buffer).await?;

            let input = String::from_utf8_lossy(&buffer);

            write_changes(&input).await?;
        }
        None => {
            let path = match &cli.path {
                Some(path) => path,
                None => anyhow::bail!("a path is required if just using noil"),
            };

            let output = get_outputs(path, cli.no_color).await?;

            let mut stdout = tokio::io::stdout();
            stdout.write_all(output.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

async fn write_changes(input: &str) -> anyhow::Result<()> {
    let noil_index = parse(input).context("parse input")?;

    fn print_op(key: &str, index: Option<&str>, path: Option<&Path>) {
        match index {
            Some(index) => match path {
                Some(path) => println!("OP: {key} ({index}) - {}", path.display()),

                None => println!("OP: {key} ({index})"),
            },
            None => match path {
                Some(path) => {
                    println!("OP: {key} - {}", path.display())
                }
                None => println!("OP: {key}"),
            },
        }
    }

    println!("Changes: \n");
    for item in noil_index.files {
        match item.entry.operation {
            Operation::Add => print_op("ADD", None, Some(&item.path)),
            Operation::Copy { index } => print_op("COPY", Some(&index), Some(&item.path)),
            Operation::Delete { index } => print_op("DELETE", Some(&index), Some(&item.path)),
            Operation::Move { index } => print_op("MOVE", Some(&index), Some(&item.path)),
            _ => {}
        }
    }
    print!("\nApply changes? (y/N): ");
    println!();

    Ok(())
}

async fn get_outputs(path: &Path, no_color: bool) -> anyhow::Result<String> {
    let mut paths = Vec::new();
    for entry in ignore::WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .ignore(true)
        .build()
    {
        let entry = entry?;

        let hash = blake3::hash(entry.path().to_string_lossy().as_bytes());

        let hash_output = encode_256bit_base36(hash.as_bytes());

        paths.push((hash_output, entry.into_path()));
    }

    paths.sort_by_key(|(h, _p)| h.clone());

    let hashes = paths.iter().map(|(h, _)| h.as_str()).collect::<Vec<&str>>();
    let (shortest_len, _global_prefixes, individual_prefixes) = shortest_unique_prefixes(&hashes);

    let mut paths = paths
        .into_iter()
        .enumerate()
        .map(|(index, (_, p))| (&_global_prefixes[index], &individual_prefixes[index], p))
        .collect::<Vec<_>>();

    paths.sort_by_key(|(_, _h, p)| p.clone());

    let mut lines = Vec::new();

    for (prefix, individual_prefix, path) in paths {
        let path_str = path.display().to_string();
        let mut line = String::new();
        write!(
            &mut line,
            "   {}{}   :   {}{}",
            {
                if no_color {
                    prefix
                } else if let Some(suffix) = prefix.strip_prefix(individual_prefix) {
                    //&format!("*{individual_prefix}*{suffix}")
                    &format!("{individual_prefix}{suffix}")
                } else {
                    prefix
                }
            },
            " ".repeat(shortest_len - prefix.len()),
            path_str,
            {
                if path.is_dir() && !path.to_string_lossy().trim_end().ends_with("/") {
                    "/"
                } else {
                    ""
                }
            }
        )?;

        lines.push(line);
    }

    Ok(lines.join("\n"))
}

fn format(input: &str) -> anyhow::Result<String> {
    let noil_index = parse(input).context("parse input")?;

    let max_op_len = noil_index
        .files
        .iter()
        .map(|f| f.entry.operation.to_string().len())
        .max()
        .unwrap_or_default();
    let max_prefix_len = noil_index
        .files
        .iter()
        .map(|f| match &f.entry.operation {
            Operation::Copy { index }
            | Operation::Delete { index }
            | Operation::Move { index }
            | Operation::Existing { index } => index.len(),
            Operation::Add => 0,
        })
        .max()
        .unwrap_or_default();

    let mut output_buf = Vec::new();

    for file in noil_index.files {
        let mut line = String::new();
        let space = " ";

        // Write operation
        let operation = file.entry.operation.to_string();
        if !operation.is_empty() {
            let spaces = max_op_len - operation.len();
            line.write_str(&operation)?;
            line.write_str(&space.repeat(spaces))?;
        } else {
            line.write_str(&space.repeat(max_op_len))?;
        }
        if max_op_len > 0 {
            line.write_str(&space.repeat(3))?;
        }

        // Write index
        let index = match file.entry.operation {
            Operation::Copy { index }
            | Operation::Delete { index }
            | Operation::Move { index }
            | Operation::Existing { index } => Some(index),
            Operation::Add => None,
        };

        if let Some(index) = index {
            let spaces = max_prefix_len - index.len();
            line.write_str(&index)?;
            line.write_str(&space.repeat(spaces))?;
        } else {
            line.write_str(&space.repeat(max_prefix_len))?;
        }
        if max_prefix_len > 0 {
            line.write_str(&space.repeat(3))?;
        }

        // Write divider
        line.write_str(":")?;
        line.write_str(&space.repeat(3))?;

        // Write path
        line.write_str(&file.path.display().to_string())?;

        output_buf.push(line);
    }

    let output = output_buf.join("\n");

    Ok(output)
}

#[cfg(test)]
mod test_format {
    #[test]
    fn can_format_complex_file() -> anyhow::Result<()> {
        let input = r#"
C asdf : /Somasdlf
as    :      /bla/bla/bla
MOVE assdfasdf    :    /bla/bla/bla
RENAME asdf23 : /bla/bla/bla
a : /bla/bla/bla

123 :     /123123/1231
        "#;

        let expected = r#"
COPY   asdf        :   /Somasdlf
       as          :   /bla/bla/bla
MOVE   assdfasdf   :   /bla/bla/bla
MOVE   asdf23      :   /bla/bla/bla
       a           :   /bla/bla/bla
       123         :   /123123/1231
        "#
        .trim();

        let output = super::format(input)?;

        pretty_assertions::assert_eq!(expected, &output);

        Ok(())
    }

    #[test]
    fn can_format_no_op() -> anyhow::Result<()> {
        let input = r#"
asdf : /Somasdlf
as    :      /bla/bla/bla
   assdfasdf    :    /bla/bla/bla
 asdf23 : /bla/bla/bla
a : /bla/bla/bla

123 :     /123123/1231
        "#;

        let expected = r#"
asdf        :   /Somasdlf
as          :   /bla/bla/bla
assdfasdf   :   /bla/bla/bla
asdf23      :   /bla/bla/bla
a           :   /bla/bla/bla
123         :   /123123/1231
        "#
        .trim();

        let output = super::format(input)?;

        pretty_assertions::assert_eq!(expected, &output);

        Ok(())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Buffer {
    files: Vec<File>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct File {
    path: PathBuf,
    entry: FileEntry,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FileEntry {
    raw_op: Option<String>,
    operation: Operation,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Operation {
    Existing { index: String },
    Add,
    Copy { index: String },
    Delete { index: String },
    Move { index: String },
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            Operation::Existing { .. } => "",
            Operation::Add => "ADD",
            Operation::Copy { .. } => "COPY",
            Operation::Delete { .. } => "DELETE",
            Operation::Move { .. } => "MOVE",
        };

        f.write_str(op)
    }
}

impl FileEntry {
    fn parse(file_entry: &str) -> anyhow::Result<Self> {
        let items = file_entry.split(' ').collect::<Vec<_>>();

        // get left most non-empty
        let Some(first) = items.first() else {
            anyhow::bail!("not a valid file entry, doesn't contain anything");
        };

        let Some(last) = items.last() else {
            anyhow::bail!("not a valid file entry, doesn't contain anything");
        };

        if first == last && !first.chars().any(|c| c.is_uppercase()) {
            // We've got a raw index

            return Ok(Self {
                raw_op: None,
                operation: Operation::Existing {
                    index: first.to_string(),
                },
            });
        }

        let index = last.to_string();

        let op = match *first {
            // ADD: first == last is sanity check there there is nothing else for this operation
            "A" | "ADD" if first == last => Operation::Add {},
            // COPY: First cannot be equal last here, otherwise there is no index
            "C" | "COPY" if first != last => Operation::Copy { index },
            // DELETE:
            "D" | "DEL" | "DELETE" if first != last => Operation::Delete { index },
            // MOVE:
            "M" | "MV" | "MOVE" | "RENAME" if first != last => Operation::Move { index },
            o => {
                anyhow::bail!("operation: {} is not supported", o);
            }
        };

        Ok(FileEntry {
            raw_op: Some(first.to_string()),
            operation: op,
        })
    }
}

#[cfg(test)]
mod test_2 {
    use crate::{parse, Buffer, File, FileEntry};

    #[test]
    fn can_parse_item() -> anyhow::Result<()> {
        let input = r#"
abc  : /var/my                
ecd  : /var/my/path                
"#;

        let output = parse(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "abc".into()
                            },
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "ecd".into()
                            }
                        }
                    }
                ]
            },
            output
        );

        Ok(())
    }

    #[test]
    fn can_parse_item_add_operation() -> anyhow::Result<()> {
        let input = r#"
abc  : /var/my                
ecd  : /var/my/path                
A    : /var/my/path/new-path                
ADD  : /var/my/path/new-long-path                
"#;

        let output = parse(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path/new-path".into(),
                        entry: FileEntry {
                            raw_op: Some("A".into()),
                            operation: crate::Operation::Add,
                        },
                    },
                    File {
                        path: "/var/my/path/new-long-path".into(),
                        entry: FileEntry {
                            raw_op: Some("ADD".into()),
                            operation: crate::Operation::Add,
                        }
                    }
                ]
            },
            output
        );

        Ok(())
    }

    #[test]
    fn can_parse_item_copy_operation() -> anyhow::Result<()> {
        let input = r#"
abc      : /var/my                
ecd      : /var/my/path                
C    abc : /var/my/path/copy-into                
COPY ecd : /var/my/path/copy-into-long                
"#;

        let output = parse(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path/copy-into".into(),
                        entry: FileEntry {
                            raw_op: Some("C".into()),
                            operation: crate::Operation::Copy {
                                index: "abc".into()
                            },
                        },
                    },
                    File {
                        path: "/var/my/path/copy-into-long".into(),
                        entry: FileEntry {
                            raw_op: Some("COPY".into()),
                            operation: crate::Operation::Copy {
                                index: "ecd".into()
                            },
                        }
                    }
                ]
            },
            output
        );

        Ok(())
    }
    #[test]
    fn can_parse_item_delete_operation() -> anyhow::Result<()> {
        let input = r#"
D abc           : /var/my                
DEL ecd         : /var/my/path                
DELETE ecd      : /var/my/path                
"#;

        let output = parse(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: Some("D".into()),
                            operation: crate::Operation::Delete {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: Some("DEL".into()),
                            operation: crate::Operation::Delete {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: Some("DELETE".into()),
                            operation: crate::Operation::Delete {
                                index: "ecd".into()
                            }
                        },
                    },
                ]
            },
            output
        );

        Ok(())
    }

    #[test]
    fn can_parse_item_move_operation() -> anyhow::Result<()> {
        let input = r#"
abc        : /var/my
ecd        : /var/my/path
M abc      : /var/my/some-different-place
MV ecd     : /var/my/some-different-place
MOVE ecd   : /var/my/some-different-place
RENAME ecd : /var/my/some-different-place
"#;

        let output = parse(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: crate::Operation::Existing {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("M".into()),
                            operation: crate::Operation::Move {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("MV".into()),
                            operation: crate::Operation::Move {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("MOVE".into()),
                            operation: crate::Operation::Move {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("RENAME".into()),
                            operation: crate::Operation::Move {
                                index: "ecd".into()
                            }
                        },
                    },
                ]
            },
            output
        );

        Ok(())
    }
}

fn parse(input: &str) -> anyhow::Result<Buffer> {
    let mut files = Vec::default();
    // We are keeping parsing simple. For each line take any non empty lines, the first part should be an index. This is where the magic happens, if it contains special tokens handle accordingly, the path always comes after a :.
    for line in input.lines() {
        if let Some((left, right)) = line.trim().rsplit_once(" : ") {
            let path = PathBuf::from(right.trim());
            let file_entry = FileEntry::parse(left.trim())?;

            files.push(File {
                path,
                entry: file_entry,
            })
        }
    }

    Ok(Buffer { files })
}

fn shortest_unique_prefixes(values: &[&str]) -> (usize, Vec<String>, Vec<String>) {
    if values.is_empty() {
        return (0, Vec::new(), Vec::new());
    }

    let len = values[0].len();
    let mut global_prefix_len = 0;
    let mut individual_prefixes = Vec::with_capacity(values.len());

    // Helper to find shared prefix length
    fn shared_prefix_len(a: &str, b: &str) -> usize {
        a.chars()
            .zip(b.chars())
            .take_while(|(ac, bc)| ac == bc)
            .count()
    }

    for i in 0..values.len() {
        let cur = values[i];
        let mut max_shared = 0;

        if i > 0 {
            max_shared = max_shared.max(shared_prefix_len(cur, values[i - 1]));
        }
        if i + 1 < values.len() {
            max_shared = max_shared.max(shared_prefix_len(cur, values[i + 1]));
        }

        // Add 1 to ensure uniqueness
        let unique_len = (max_shared + 1).min(len);
        individual_prefixes.push(cur[..unique_len].to_string());

        // For global prefix: max shared between any two neighbors
        if i + 1 < values.len() {
            global_prefix_len = global_prefix_len.max(shared_prefix_len(cur, values[i + 1]) + 1);
        }
    }

    global_prefix_len = global_prefix_len.min(len);
    let global_prefixes = values
        .iter()
        .map(|s| s[..global_prefix_len].to_string())
        .collect();

    (global_prefix_len, global_prefixes, individual_prefixes)
}

#[cfg(test)]
mod test {
    use crate::shortest_unique_prefixes;

    #[test]
    fn simple_shortest() {
        let mut input = vec!["1ab", "3ab", "1ca"];
        let expected_len: usize = 2;
        let expected_global: Vec<String> = vec!["1a".into(), "1c".into(), "3a".into()];
        let expected_individual: Vec<String> = vec!["1a".into(), "1c".into(), "3".into()];

        input.sort();

        let (len, global_prefixes, individual_prefixes) = shortest_unique_prefixes(&input);

        assert_eq!(expected_len, len);
        assert_eq!(expected_global, global_prefixes);
        assert_eq!(expected_individual, individual_prefixes);
    }
}
