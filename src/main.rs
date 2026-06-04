use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

mod command;
mod utils;

use crate::utils::{compress_zlib, compute_sha1, decompress_zlib};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Usage: ...");
        return Ok(());
    }

    if args[1] == "init" {
        command::init::run()?;
    } else if args[1] == "cat-file" {
        assert!(args.len() == 4 && args[2] == "-p");
        let blob_sha = args[3].as_str();
        command::cat_file::run(blob_sha)?;
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
        command::hash_object::run(&file_path, write_flag)?;
    } else {
        println!("unknown command: {}", args[1])
    }

    Ok(())
}
