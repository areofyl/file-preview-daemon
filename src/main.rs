mod config;
mod copy;
mod drag;
mod state;
mod status;
mod watch;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "file-preview", about = "File preview daemon for Waybar")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the inotify watcher daemon
    Watch,
    /// Output status JSON for Waybar
    Status,
    /// Copy latest file path to clipboard via wl-copy
    Copy,
    /// Launch drag-and-drop overlay at cursor
    Drag,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::Config::load()?;

    match cli.command {
        Commands::Watch => watch::run(&cfg),
        Commands::Status => status::run(&cfg),
        Commands::Copy => copy::run(&cfg),
        Commands::Drag => drag::run(&cfg),
    }
}
