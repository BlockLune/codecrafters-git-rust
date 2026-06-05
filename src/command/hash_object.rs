use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::object::blob::BlobObject;

pub fn run(file_path: &Path, write_flag: bool) -> Result<()> {
    let file_content = fs::read(file_path)?;
    let blob_obj = BlobObject::new(&file_content);
    println!("{}", blob_obj.sha1()?);
    if write_flag {
        blob_obj.write_to_disk()?;
    }

    Ok(())
}
