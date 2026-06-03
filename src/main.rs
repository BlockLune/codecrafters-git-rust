use anyhow::Result;
use flate2::bufread::ZlibDecoder;

use std::env;
use std::fs;
use std::io::Read;
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
