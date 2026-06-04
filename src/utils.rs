use anyhow::Result;
use flate2::Compression;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};

pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>> {
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

pub fn split_header_content(decompressed: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
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
