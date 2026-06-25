use anyhow::{Result, ensure};
use std::fs;
use std::path::Path;

use crate::util::compression::compress_zlib;

pub fn write_loose_object(root: &Path, sha1: &[u8], data: &[u8]) -> Result<()> {
    let obj_sha_hex = hex::encode(sha1);
    let (dir, filename) = obj_sha_hex.split_at(2);
    let dir_path = root.join(".git/objects/").join(dir);
    write_file_create_parent(&dir_path, filename, &compress_zlib(data)?)
}

pub fn write_head_symref(root: &Path, symref_head: &str) -> Result<()> {
    let dir_path = root.join(".git");
    write_file_create_parent(
        &dir_path,
        "HEAD",
        format!("ref: {}\n", symref_head).as_bytes(),
    )
}

pub fn write_branch_ref(root: &Path, head_name: &str, sha1: &[u8]) -> Result<()> {
    const PREFIX: &str = "refs/heads/";
    const PREFIX_LEN: usize = PREFIX.len();
    ensure!(head_name.starts_with(PREFIX));
    let dir_path = root.join(".git").join(PREFIX);
    let filename = &head_name[PREFIX_LEN..];
    write_file_create_parent(
        &dir_path,
        filename,
        format!("{}\n", hex::encode(sha1)).as_bytes(),
    )
}

fn write_file_create_parent(dir_path: &Path, filename: &str, data: &[u8]) -> Result<()> {
    fs::create_dir_all(&dir_path)?;
    let path = dir_path.join(filename);
    fs::write(path, data)?;
    Ok(())
}
