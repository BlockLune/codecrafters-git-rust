use anyhow::{Result, bail};
use std::fs;
use std::path::PathBuf;

use crate::utils::{decompress_zlib, split_header_content};

pub fn run(blob_sha: &str, pretty_print_flag: bool) -> Result<()> {
    if !pretty_print_flag {
        bail!("only -p mode is supported");
    }
    let (dir, filename) = blob_sha.split_at(2);
    let path = PathBuf::from(".git/objects/").join(dir).join(filename);
    let data = fs::read(path)?;
    let decompressed = decompress_zlib(&data)?;
    let (_, content) = split_header_content(&decompressed)?;
    print!("{}", String::from_utf8_lossy(&content));

    Ok(())
}
