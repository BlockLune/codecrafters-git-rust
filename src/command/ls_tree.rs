use anyhow::{Context, Result, bail, ensure};

use crate::object::tree::TreeEntry;
use crate::util::get_decompressed_header_content_from_sha;

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
