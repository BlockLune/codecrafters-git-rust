use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{compress_zlib, compute_sha1};

pub fn run(file_path: &Path, write_flag: bool) -> Result<()> {
    let file_content = fs::read(file_path)?;
    let mut data = Vec::from(format!("blob {}\0", file_content.len()).as_bytes());
    data.extend_from_slice(&file_content);
    let sha1 = compute_sha1(&data)?;
    println!("{}", sha1);

    if write_flag {
        let (dir, filename) = sha1.split_at(2);
        let path = PathBuf::from(".git/objects/").join(dir).join(filename);
        fs::write(path, compress_zlib(&data)?)?;
    }

    Ok(())
}
