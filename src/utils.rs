use anyhow::Result;
use flate2::Compression;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};

pub fn decompress_zlib(data: &[u8]) -> Result<String> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
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
