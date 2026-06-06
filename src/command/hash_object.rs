use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::object::GitObject;
use crate::object::blob::BlobObject;

pub fn run(file_path: &Path, write_flag: bool) -> Result<()> {
    let file_content = fs::read(file_path)?;
    let blob_obj = BlobObject::new(&file_content);
    let blob_obj_sha1_hex = hex::encode(blob_obj.sha1());
    println!("{}", blob_obj_sha1_hex);
    if write_flag {
        blob_obj.write_to_disk()?;
    }

    Ok(())
}
