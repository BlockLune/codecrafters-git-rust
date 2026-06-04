use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    CatFile {
        #[arg(short = 'p')]
        blob_sha: String,
    },
    HashObject {
        #[arg(short = 'w')]
        write_flag: bool,
        file_path: PathBuf,
    },
}
