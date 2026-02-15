use crate::config::Config;
use crate::state::read_history;
use anyhow::Result;
use std::process::Command;

pub fn run(cfg: &Config) -> Result<()> {
    let history = read_history(&Config::state_file());
    if let Some(st) = history.current() {
        if !st.is_expired(cfg.dismiss_seconds) && st.path.exists() {
            Command::new("wl-copy")
                .arg(st.path.to_string_lossy().as_ref())
                .output()?;
        }
    }
    Ok(())
}
