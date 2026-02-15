use crate::config::Config;
use crate::state::read_state;
use anyhow::Result;
use std::process::Command;

pub fn run(cfg: &Config) -> Result<()> {
    let state_file = Config::state_file();
    if let Some(st) = read_state(&state_file, cfg.dismiss_seconds) {
        if st.path.exists() {
            Command::new("wl-copy")
                .arg(st.path.to_string_lossy().as_ref())
                .output()?;
        }
    }
    Ok(())
}
