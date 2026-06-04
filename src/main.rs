use anyhow::Result;
use clap::Parser;

mod cli;
mod command;
mod utils;

use crate::cli::{Cli, Commands};
use crate::utils::{compress_zlib, compute_sha1};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => command::init::run(),
        Commands::CatFile {
            pretty_print_flag,
            blob_sha,
        } => command::cat_file::run(&blob_sha, pretty_print_flag),
        Commands::HashObject {
            write_flag,
            file_path,
        } => command::hash_object::run(&file_path, write_flag),
        Commands::LsTree {
            name_only_flag,
            tree_sha,
        } => command::ls_tree::run(&tree_sha, name_only_flag),
    }?;

    Ok(())
}
