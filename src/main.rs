use anyhow::Result;
use clap::Parser;

mod cli;
mod command;
mod utils;
mod object;

use crate::cli::{Cli, Commands};

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
        Commands::WriteTree => command::write_tree::run(),
    }?;

    Ok(())
}
