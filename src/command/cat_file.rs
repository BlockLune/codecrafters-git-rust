use anyhow::{Result, bail};

use crate::util::get_decompressed_header_content_from_sha;

pub fn run(blob_sha: &str, pretty_print_flag: bool) -> Result<()> {
    if !pretty_print_flag {
        bail!("only -p mode is supported");
    }
    let (_, content) = get_decompressed_header_content_from_sha(blob_sha)?;
    print!("{}", String::from_utf8_lossy(&content));

    Ok(())
}
