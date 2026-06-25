use anyhow::{Result, bail};
use std::path::PathBuf;

use crate::object::tree::parse_tree_entries;
use crate::util::get_decompressed_header_content_from_sha;

pub fn run(tree_sha: &str, name_only_flag: bool) -> Result<()> {
    if !name_only_flag {
        bail!("only --name-only mode is supported");
    }
    let (_, content) = get_decompressed_header_content_from_sha(&PathBuf::from("."), tree_sha)?;
    let tree_entries = parse_tree_entries(&content)?;

    for tree_entry in tree_entries {
        println!("{}", String::from_utf8_lossy(&tree_entry.name));
    }

    Ok(())
}
