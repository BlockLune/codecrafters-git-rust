use anyhow::Result;
use flate2::Compression;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed: Vec<u8> = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

pub fn compress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

pub fn compute_sha1(data: &[u8]) -> Result<String> {
    Ok(hex::encode(Sha1::digest(data)))
}

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
    let decompressed = decompress_zlib(&data)?;
    split_header_content(&decompressed)
}
