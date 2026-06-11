use anyhow::{Result, bail};
use std::fs;
use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub(crate) mod compression;
pub(crate) mod pkt_line;

fn split_header_content(decompressed: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut header: Vec<u8> = Vec::new();
    let mut content: Vec<u8> = Vec::new();
    let mut header_finished = false;

    for byte in decompressed {
        if *byte == b'\0' {
            header_finished = true;
            continue;
        }

        if header_finished {
            content.push(*byte);
        } else {
            header.push(*byte);
        }
    }

    Ok((header, content))
}

pub fn get_decompressed_header_content_from_sha(obj_sha: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let (dir, filename) = obj_sha.split_at(2);
    let path = PathBuf::from(".git/objects/").join(dir).join(filename);
    let data = fs::read(path)?;
    let (decompressed, _) = compression::decompress_zlib(&data)?;
    split_header_content(&decompressed)
}

#[cfg(unix)]
pub fn git_mode(metadata: &Metadata) -> Result<String> {
    if metadata.is_dir() {
        return Ok("40000".to_string());
    }

    if metadata.is_symlink() {
        return Ok("120000".to_string());
    }

    if metadata.is_file() {
        let perm = metadata.permissions().mode();
        if perm & 0o111 != 0 {
            return Ok("100755".to_string());
        } else {
            return Ok("100644".to_string());
        }
    }

    bail!("unsupported file type");
}
