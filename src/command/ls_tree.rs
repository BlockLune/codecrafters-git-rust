use anyhow::{Result, bail};

use crate::object::tree::TreeEntry;
use crate::utils::get_decompressed_header_content_from_sha;

pub fn run(tree_sha: &str, name_only_flag: bool) -> Result<()> {
    if !name_only_flag {
        bail!("only --name-only mode is supported");
    }
    let (_, content) = get_decompressed_header_content_from_sha(tree_sha)?;
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

        let sha_20 = Vec::from(&content[null_pos + 1..=null_pos + 20]);

        let entry = TreeEntry { mode, name, sha_20 };
        entries.push(entry);

        i = null_pos + 1 + 20
    }

    Ok(entries)
}
