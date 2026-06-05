use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::object::blob::BlobObject;
use crate::object::tree::{TreeEntry, TreeObject};
use crate::utils::git_mode;

pub fn run() -> Result<()> {
    let sha1 = write_tree_for_dir(Path::new("."))?;
    println!("{}", hex::encode(sha1));
    Ok(())
}

fn write_tree_for_dir(path: &Path) -> Result<Vec<u8>> {
    let mut tree_entries = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name();

        if name.to_string_lossy() == ".git" {
            continue;
        }

        let path = entry.path();
        let metadata = entry.metadata()?;
        let mode = git_mode(&metadata)?;

        let tree_entry = if metadata.is_dir() {
            let sha1 = write_tree_for_dir(&path)?;
            TreeEntry::new(mode.as_bytes(), name.as_encoded_bytes(), &sha1)
        } else {
            let content = fs::read(&path)?;
            let blob = BlobObject::new(&content);
            let sha1 = blob.sha1();
            blob.write_to_disk()?;
            TreeEntry::new(mode.as_bytes(), name.as_encoded_bytes(), &sha1)
        };

        tree_entries.push(tree_entry);
    }

    // git requires all entries are alphabetically sorted by name
    tree_entries.sort_by(|a, b| a.name.cmp(&b.name));

    let tree = TreeObject::new(&tree_entries);
    let sha1 = tree.sha1();
    tree.write_to_disk()?;

    Ok(sha1)
}
