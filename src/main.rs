mod config;
mod copy;
mod drag;
mod init;
mod menu;
mod scroll;
mod state;
mod status;
mod watch;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "glance", about = "A file clipboard for Wayland")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the inotify watcher daemon
    Watch,
    /// Output status JSON for Waybar
    Status {
        #[arg(long)]
        index: Option<usize>,
    },
    /// Copy latest file path to clipboard via wl-copy
    Copy,
    /// Launch drag-and-drop overlay at cursor
    Drag,
    /// Show dropdown menu below Waybar with actions
    Menu,
    /// Scroll through file history (up/down)
    Scroll {
        direction: String,
    },
    /// Set up config, Waybar module, CSS, and Hyprland autostart
    Init,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if matches!(cli.command, Commands::Init) {
        return init::run();
    }

    let cfg = config::Config::load()?;

    match cli.command {
        Commands::Watch => watch::run(&cfg),
        Commands::Status { index } => status::run(&cfg, index),
        Commands::Copy => copy::run(&cfg),
        Commands::Drag => drag::run(&cfg),
        Commands::Menu => menu::run(&cfg),
        Commands::Scroll { ref direction } => scroll::run(&cfg, direction),
        Commands::Init => unreachable!(),
    }
}
