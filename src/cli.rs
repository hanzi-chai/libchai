use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, value_name = "FILE")]
    pub config: PathBuf,

    #[arg(short, long, value_name = "FILE")]
    pub elements: PathBuf
}

#[derive(Subcommand)]
enum Commands {
    /// Optimize your schema
    Optimize {},
}
