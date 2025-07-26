use std::path::Path;

use anyhow::Context;

use crate::{models::Operation, parse::parse};

pub async fn write_changes(input: &str) -> anyhow::Result<()> {
    let noil_index = parse(input).context("parse input")?;

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
    print!("\nApply changes? (y/N): ");
    println!();

    Ok(())
}
