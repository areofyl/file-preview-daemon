use crate::config::Config;
use crate::state::{read_history, write_history};
use anyhow::Result;
use std::process::Command;

pub fn run(cfg: &Config, direction: &str) -> Result<()> {
    let state_file = Config::state_file();
    let mut history = read_history(&state_file);
    match direction {
        "up" => history.select_prev(),
        "down" => history.select_next(),
        _ => {}
    }
    write_history(&state_file, &history)?;
    let _ = Command::new("pkill")
        .arg(format!("-RTMIN+{}", cfg.signal_number))
        .arg("waybar")
        .output();
    Ok(())
}
