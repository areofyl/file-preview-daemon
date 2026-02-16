use crate::config::Config;
use crate::state::with_history;
use anyhow::Result;
use std::process::Command;

pub fn run(cfg: &Config, direction: &str) -> Result<()> {
    let state_file = Config::state_file();
    let dir = direction.to_string();
    with_history(&state_file, |history| {
        match dir.as_str() {
            "up" => history.select_next(),
            "down" => history.select_prev(),
            _ => {}
        }
    })?;
    let _ = Command::new("pkill")
        .arg(format!("-RTMIN+{}", cfg.signal_number))
        .arg("waybar")
        .output();
    Ok(())
}
