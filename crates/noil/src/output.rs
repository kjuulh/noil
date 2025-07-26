use std::{fmt::Write, path::Path};

use crate::{encode_rand, find_prefix};

pub async fn get_outputs(path: &Path, no_color: bool) -> anyhow::Result<String> {
    let mut paths = Vec::new();
    for entry in ignore::WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .ignore(true)
        .build()
    {
        let entry = entry?;

        let hash = blake3::hash(entry.path().to_string_lossy().as_bytes());

        let hash_output = encode_rand::encode_256bit_base36(hash.as_bytes());

        paths.push((hash_output, entry.into_path()));
    }

    paths.sort_by_key(|(h, _p)| h.clone());

    let hashes = paths.iter().map(|(h, _)| h.as_str()).collect::<Vec<&str>>();
    let (shortest_len, _global_prefixes, individual_prefixes) =
        find_prefix::shortest_unique_prefixes(&hashes);

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
