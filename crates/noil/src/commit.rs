use std::path::Path;

use ansi_term::Color;
use anyhow::Context;
use tokio::io::{AsyncBufReadExt, BufReader};

use std::io::Write;

use crate::{models::Operation, parse::parse_input};

pub enum Action {
    Quit,
    Apply { original: String },
    Edit,
}

pub async fn print_changes(input: &str, preview: bool) -> anyhow::Result<Action> {
    let noil_index = parse_input(input).context("parse input")?;

    fn print_op(key: &str, index: Option<&str>, path: Option<&Path>) {
        match index {
            Some(index) => match path {
                Some(path) => println!("  - {key} ({index}) - {}", path.display()),

                None => println!("  - {key} ({index})"),
            },
            None => match path {
                Some(path) => {
                    println!("  - {key} - {}", path.display())
                }
                None => println!("  - {key}"),
            },
        }
    }

    eprintln!("Changes:\n");

    for item in noil_index.files {
        match item.entry.operation {
            Operation::Add => print_op(
                &format!("{}", ansi_term::Color::Green.bold().paint("ADD")),
                None,
                Some(&item.path),
            ),
            Operation::Copy { index } => print_op(
                &format!("{}", ansi_term::Color::Blue.bold().paint("COPY")),
                Some(&index),
                Some(&item.path),
            ),
            Operation::Delete { index } => print_op(
                &format!("{}", ansi_term::Color::Red.bold().paint("DELETE")),
                Some(&index),
                Some(&item.path),
            ),
            Operation::Move { index } => print_op(
                &format!(
                    "{}",
                    // Orange
                    ansi_term::Color::RGB(224, 145, 64).bold().paint("MOVE")
                ),
                Some(&index),
                Some(&item.path),
            ),
            _ => {}
        }
    }

    if preview {
        return Ok(Action::Quit);
    }

    eprint!("\nApply changes? (y (yes) / n (abort) / E (edit)): ");
    let mut stderr = std::io::stderr();
    stderr.flush()?;

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input_buf = String::new();

    reader.read_line(&mut input_buf).await?;
    let trimmed = input_buf.trim().to_lowercase();

    match trimmed.as_str() {
        "y" => {
            println!("Confirmed.");

            Ok(Action::Apply {
                original: input.to_string(),
            })
        }
        "n" => {
            println!("Aborted.");
            Ok(Action::Quit)
        }
        "e" | "" => {
            println!("Edit");

            Ok(Action::Edit)
        }
        _ => {
            println!("Invalid input: {}", Color::Red.normal().paint(trimmed));

            eprint!("press enter to edit: ");
            let mut stderr = std::io::stderr();
            stderr.flush()?;

            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut input_buf = String::new();
            reader.read_line(&mut input_buf).await?;

            Ok(Action::Edit)
        }
    }
}
