use anyhow::{Context, Result, ensure};

use crate::object::GitObject;

pub struct TreeEntry {
    #[allow(unused)]
    pub mode: Vec<u8>,
    pub name: Vec<u8>,
    pub sha1_20: Vec<u8>,
}

impl TreeEntry {
    pub fn new(mode: &[u8], name: &[u8], sha: &[u8]) -> Self {
        Self {
            mode: Vec::from(mode),
            name: Vec::from(name),
            sha1_20: Vec::from(&sha[..20]),
        }
    }
}

pub struct TreeObject(Vec<u8>);

impl TreeObject {
    pub fn new(entries: &[TreeEntry]) -> Self {
        let mut content: Vec<u8> = Vec::new();
        for entry in entries {
            content.extend_from_slice(&entry.mode);
            content.push(b' ');
            content.extend_from_slice(&entry.name);
            content.push(b'\0');
            content.extend_from_slice(&entry.sha1_20);
        }

        let mut data = Vec::from(format!("tree {}\0", content.len()).as_bytes());
        data.extend_from_slice(&content);

        Self(data)
    }
}

impl GitObject for TreeObject {
    fn data(&self) -> &[u8] {
        &self.0
    }
}

pub fn parse_tree_entries(content: &[u8]) -> Result<Vec<TreeEntry>> {
    let mut entries: Vec<TreeEntry> = Vec::new();
    let mut i = 0;

    while i < content.len() {
        let space_pos = i + content[i..]
            .iter()
            .position(|byte| *byte == b' ')
            .context("failed to parse tree entry: missing mode separator")?;
        let mode = Vec::from(&content[i..space_pos]);

        let name_start = space_pos + 1;
        let null_pos = name_start
            + content[name_start..]
                .iter()
                .position(|byte| *byte == b'\0')
                .context("failed to parse tree entry: missing name terminator")?;
        let name = Vec::from(&content[name_start..null_pos]);

        let sha_start = null_pos + 1;
        let sha_end = sha_start + 20;
        ensure!(
            sha_end <= content.len(),
            "failed to parse tree entry: truncated sha"
        );
        let sha1_20 = Vec::from(&content[sha_start..sha_end]);

        let entry = TreeEntry {
            mode,
            name,
            sha1_20,
        };
        entries.push(entry);

        i = sha_end;
    }

    Ok(entries)
}
