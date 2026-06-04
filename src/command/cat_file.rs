use anyhow::Result;
use std::fs;
use std::path::PathBuf;

use crate::decompress_zlib;

pub fn run(blob_sha: &str) -> Result<()> {
    let (dir, filename) = blob_sha.split_at(2);
    let path = PathBuf::from(".git/objects/").join(dir).join(filename);
    let data = fs::read(path).unwrap();
    let decompressed: Vec<_> = decompress_zlib(&data)?
        .splitn(2, '\0')
        .map(String::from)
        .collect();
    let content = &decompressed[1];
    print!("{}", content);

    Ok(())
}
