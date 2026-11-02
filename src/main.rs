#[cfg(not(target_os = "macos"))]
compile_error!("macos-history-diff only supports macOS");

mod commands;
mod diff;
mod selector;
mod versions;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "mhd",
    version,
    about = "Show how a text file evolved using macOS file versions"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List saved versions of a file
    List { file: PathBuf },
    /// Diff two versions (defaults: second newest vs current)
    Diff {
        file: PathBuf,
        /// Version index from `list` (0 = oldest) or `current`
        a: Option<String>,
        /// Version index from `list` (0 = oldest) or `current`
        b: Option<String>,
    },
    /// Show successive diffs through the file's history
    Log {
        file: PathBuf,
        /// Show oldest first
        #[arg(long)]
        reverse: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List { file } => commands::list(&file),
        Command::Diff { file, a, b } => commands::diff(&file, a.as_deref(), b.as_deref()),
        Command::Log { file, reverse } => commands::log(&file, reverse),
    }
}
