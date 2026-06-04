use anyhow::{Result, bail};
use std::fs;
use std::path::PathBuf;

use crate::utils::{decompress_zlib, split_header_content};

#[allow(unused)]
pub struct TreeEntry {
    pub mode: Vec<u8>,
    pub name: Vec<u8>,
    pub hash_bin: Vec<u8>,
}

pub fn run(tree_sha: &str, name_only_flag: bool) -> Result<()> {
    if !name_only_flag {
        bail!("only --name-only mode is supported");
    }
    let (dir, filename) = tree_sha.split_at(2);
    let path = PathBuf::from(".git/objects/").join(dir).join(filename);
    let data = fs::read(path)?;
    let decompressed = decompress_zlib(&data)?;
    let (_, content) = split_header_content(&decompressed)?;
    let tree_entries = parse_tree_entries(&content)?;

    for tree_entry in tree_entries {
        println!("{}", String::from_utf8_lossy(&tree_entry.name));
    }

    Ok(())
}

fn parse_tree_entries(content: &[u8]) -> Result<Vec<TreeEntry>> {
    let mut entries: Vec<TreeEntry> = Vec::new();
    let mut i = 0;

    while i < content.len() {
        let (space_pos, _) = content
            .iter()
            .skip(i)
            .enumerate()
            .find(|&(_, byte)| *byte == b' ')
            .unwrap();
        let mode = Vec::from(&content[..space_pos]);

        let (null_pos, _) = content
            .iter()
            .skip(space_pos + 1)
            .enumerate()
            .find(|&(_, byte)| *byte == b'\0')
            .unwrap();
        let name = Vec::from(&content[space_pos + 1..null_pos]);

        let hash_bin = Vec::from(&content[null_pos + 1..=null_pos + 20]);

        let entry = TreeEntry {
            mode,
            name,
            hash_bin,
        };
        entries.push(entry);

        i = null_pos + 1 + 20
    }

    Ok(entries)
}
