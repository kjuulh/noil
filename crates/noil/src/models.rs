use std::fmt::Display;

use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug)]
pub struct Buffer {
    pub(crate) files: Vec<File>,
}

impl Buffer {
    pub fn get_existing(&self, index: &str) -> Option<&File> {
        self.files.iter().find(|f| match &f.entry.operation {
            Operation::Existing { index: idx } => idx == index,
            _ => false,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct File {
    pub(crate) path: PathBuf,
    pub(crate) entry: FileEntry,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FileEntry {
    pub(crate) raw_op: Option<String>,
    pub(crate) operation: Operation,
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
    pub(crate) fn parse(file_entry: &str) -> anyhow::Result<Self> {
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
pub(crate) mod test {
    use crate::{models::*, parse};

    #[test]
    fn can_parse_item() -> anyhow::Result<()> {
        let input = r#"
abc  : /var/my                
ecd  : /var/my/path                
"#;

        let output = parse::parse_input(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
                                index: "abc".into()
                            },
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
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

        let output = parse::parse_input(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path/new-path".into(),
                        entry: FileEntry {
                            raw_op: Some("A".into()),
                            operation: Operation::Add,
                        },
                    },
                    File {
                        path: "/var/my/path/new-long-path".into(),
                        entry: FileEntry {
                            raw_op: Some("ADD".into()),
                            operation: Operation::Add,
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

        let output = parse::parse_input(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path/copy-into".into(),
                        entry: FileEntry {
                            raw_op: Some("C".into()),
                            operation: Operation::Copy {
                                index: "abc".into()
                            },
                        },
                    },
                    File {
                        path: "/var/my/path/copy-into-long".into(),
                        entry: FileEntry {
                            raw_op: Some("COPY".into()),
                            operation: Operation::Copy {
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

        let output = parse::parse_input(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: Some("D".into()),
                            operation: Operation::Delete {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: Some("DEL".into()),
                            operation: Operation::Delete {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: Some("DELETE".into()),
                            operation: Operation::Delete {
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

        let output = parse::parse_input(input)?;

        pretty_assertions::assert_eq!(
            Buffer {
                files: vec![
                    File {
                        path: "/var/my".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/path".into(),
                        entry: FileEntry {
                            raw_op: None,
                            operation: Operation::Existing {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("M".into()),
                            operation: Operation::Move {
                                index: "abc".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("MV".into()),
                            operation: Operation::Move {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("MOVE".into()),
                            operation: Operation::Move {
                                index: "ecd".into()
                            }
                        },
                    },
                    File {
                        path: "/var/my/some-different-place".into(),
                        entry: FileEntry {
                            raw_op: Some("RENAME".into()),
                            operation: Operation::Move {
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
