use anyhow::Result;
use flate2::Compression;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};

use std::env;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Usage: ...");
        return Ok(());
    }

    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory")
    } else if args[1] == "cat-file" {
        assert!(args.len() == 4 && args[2] == "-p");
        let blob_sha = args[3].as_str();
        let (dir, filename) = blob_sha.split_at(2);
        let path = PathBuf::from(".git/objects/").join(dir).join(filename);
        let data = fs::read(path).unwrap();
        let decompressed: Vec<_> = decompress_zlib(&data)?
            .splitn(2, '\0')
            .map(String::from)
            .collect();
        let content = &decompressed[1];
        print!("{}", content);
    } else if args[1] == "hash-object" {
        assert!(args.len() >= 3 && args.len() <= 4);
        let mut write_flag = false;
        let mut file_path = PathBuf::new();
        for arg in &args[2..] {
            if arg == "-w" {
                write_flag = true;
                continue;
            }
            file_path = PathBuf::from(arg);
        }
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
    } else {
        println!("unknown command: {}", args[1])
    }

    Ok(())
}

fn decompress_zlib(data: &[u8]) -> Result<String> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
    Ok(decompressed)
}

fn compress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

fn compute_sha1(data: &[u8]) -> Result<String> {
    Ok(hex::encode(Sha1::digest(data)))
}
