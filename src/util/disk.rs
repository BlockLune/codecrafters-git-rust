use anyhow::{Result, ensure};
use std::fs;
use std::path::Path;

use crate::util::compression::compress_zlib;

/// Writes a loose object under `.git/objects`.
///
/// Example: object SHA `abcdxxxx...`
/// is written to `.git/objects/ab/cdxxx...`.
/// `data` must already include the Git object header, such as
/// `blob 0\0`, before it is zlib-compressed.
pub fn write_loose_object(root: &Path, sha1: &[u8], data: &[u8]) -> Result<()> {
    let obj_sha_hex = hex::encode(sha1);
    let (dir, filename) = obj_sha_hex.split_at(2);
    let path = root.join(".git/objects").join(dir).join(filename);
    write_file_create_parent(&path, &compress_zlib(data)?)
}

/// Writes `.git/HEAD`.
///
/// Example: `symref_head = "refs/heads/main"` writes:
/// `ref: refs/heads/main\n`.
pub fn write_head_symref(root: &Path, symref_head: &str) -> Result<()> {
    write_file_create_parent(
        &root.join(".git/HEAD"),
        format!("ref: {}\n", symref_head).as_bytes(),
    )
}

/// Writes a local branch ref under `.git/refs/heads`.
///
/// Example: `ref_name = "refs/heads/main"` writes the commit SHA to
/// `.git/refs/heads/main`.
pub fn write_branch_ref(root: &Path, ref_name: &str, sha1: &[u8]) -> Result<()> {
    const PREFIX: &str = "refs/heads/";
    ensure!(ref_name.starts_with(PREFIX));
    write_file_create_parent(
        &root.join(".git").join(ref_name),
        format!("{}\n", hex::encode(sha1)).as_bytes(),
    )
}

/// Writes a remote-tracking branch ref under `.git/refs/remotes/<remote>`.
///
/// Example: `remote_name = "origin"` and `ref_name = "refs/heads/main"`
/// writes the commit SHA to `.git/refs/remotes/origin/main`.
pub fn write_remote_tracking_ref(
    root: &Path,
    remote_name: &str,
    ref_name: &str,
    sha1: &[u8],
) -> Result<()> {
    let branch_name = branch_name(ref_name)?;
    write_file_create_parent(
        &root
            .join(".git/refs/remotes")
            .join(remote_name)
            .join(branch_name),
        format!("{}\n", hex::encode(sha1)).as_bytes(),
    )
}

/// Writes `.git/refs/remotes/<remote>/HEAD` as a symbolic ref.
///
/// Example: `remote_name = "origin"` and `symref_head = "refs/heads/main"`
/// writes `ref: refs/remotes/origin/main\n` to
/// `.git/refs/remotes/origin/HEAD`.
pub fn write_remote_head_symref(root: &Path, remote_name: &str, symref_head: &str) -> Result<()> {
    let branch_name = branch_name(symref_head)?;
    let remote_head = format!("refs/remotes/{}/{}", remote_name, branch_name);
    write_file_create_parent(
        &root
            .join(".git/refs/remotes")
            .join(remote_name)
            .join("HEAD"),
        format!("ref: {}\n", remote_head).as_bytes(),
    )
}

fn branch_name(ref_name: &str) -> Result<&str> {
    const PREFIX: &str = "refs/heads/";
    ensure!(ref_name.starts_with(PREFIX));
    let branch_name = &ref_name[PREFIX.len()..];
    ensure!(!branch_name.is_empty());
    Ok(branch_name)
}

fn write_file_create_parent(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, data)?;
    Ok(())
}
