use anyhow::Result;
use clap::{Parser, Subcommand};
use ndol::{apply_paths, check_paths, format_diagnostics, load_config};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check files and print diagnostics without changing files.
    Check { paths: Vec<PathBuf> },
    /// Apply safe fixes in-place.
    Apply { paths: Vec<PathBuf> },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { paths } => {
            let paths = normalize_paths(paths);
            let config = load_config(paths[0].as_path())?;
            let diagnostics = check_paths(&paths, &config)?;
            print!("{}", format_diagnostics(&diagnostics));
            if diagnostics.is_empty() {
                Ok(())
            } else {
                std::process::exit(1);
            }
        }
        Command::Apply { paths } => {
            let paths = normalize_paths(paths);
            let config = load_config(paths[0].as_path())?;
            let diagnostics = apply_paths(&paths, &config)?;
            print!("{}", format_diagnostics(&diagnostics));
            Ok(())
        }
    }
}

fn normalize_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths
    }
}
