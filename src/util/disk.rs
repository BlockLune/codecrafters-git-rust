use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::util::compression::compress_zlib;

pub fn write_to_disk(root: &Path, sha1: &[u8], data: &[u8]) -> Result<()> {
    let obj_sha_hex = hex::encode(sha1);
    let (dir, filename) = obj_sha_hex.split_at(2);
    let dir_path = root.join(".git/objects/").join(dir);
    fs::create_dir_all(&dir_path)?;
    let path = dir_path.join(filename);
    fs::write(path, compress_zlib(data)?)?;

    Ok(())
}
