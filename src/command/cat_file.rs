use anyhow::{Result, bail};
use std::path::PathBuf;

use crate::util::get_decompressed_header_content_from_sha;

pub fn run(blob_sha: &str, pretty_print_flag: bool) -> Result<()> {
    if !pretty_print_flag {
        bail!("only -p mode is supported");
    }
    let (_, content) = get_decompressed_header_content_from_sha(&PathBuf::from("."), blob_sha)?;
    print!("{}", String::from_utf8_lossy(&content));

    Ok(())
}
