use std::path::PathBuf;

use crate::models;

pub(crate) fn parse_input(input: &str) -> anyhow::Result<models::Buffer> {
    let mut files = Vec::default();
    // We are keeping parsing simple. For each line take any non empty lines, the first part should be an index. This is where the magic happens, if it contains special tokens handle accordingly, the path always comes after a :.
    for line in input.lines() {
        if let Some((left, right)) = line.trim().rsplit_once(" : ") {
            let path = PathBuf::from(right.trim());
            let file_entry = models::FileEntry::parse(left.trim())?;

            files.push(models::File {
                path,
                entry: file_entry,
            })
        }
    }

    Ok(models::Buffer { files })
}
