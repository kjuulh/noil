use std::fmt::Write;

use anyhow::Context;

use crate::models;

use super::parse::parse_input;

pub(crate) fn format(input: &str) -> anyhow::Result<String> {
    let noil_index = parse_input(input).context("parse input")?;

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
            models::Operation::Copy { index }
            | models::Operation::Delete { index }
            | models::Operation::Move { index }
            | models::Operation::Existing { index } => index.len(),
            models::Operation::Add => 0,
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
            models::Operation::Copy { index }
            | models::Operation::Delete { index }
            | models::Operation::Move { index }
            | models::Operation::Existing { index } => Some(index),
            models::Operation::Add => None,
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
pub(crate) mod test_format {
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
