use anyhow::Result;
use clap::Parser;
use std::fs;

mod cli;
mod command;
mod utils;

use crate::cli::{Cli, Commands};
use crate::utils::{compress_zlib, compute_sha1, decompress_zlib};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => command::init::run(),
        Commands::CatFile { blob_sha } => command::cat_file::run(&blob_sha),
        Commands::HashObject {
            write_flag,
            file_path,
        } => command::hash_object::run(&file_path, write_flag),
    }?;

    Ok(())
}
