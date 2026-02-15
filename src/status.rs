use crate::config::Config;
use crate::state::read_state;
use anyhow::Result;
use serde_json::json;

pub fn run(cfg: &Config) -> Result<()> {
    let state_file = Config::state_file();
    let output = match read_state(&state_file, cfg.dismiss_seconds) {
        Some(st) => {
            let name = if st.name.len() > 18 {
                format!("{}\u{2026}", &st.name[..15])
            } else {
                st.name.clone()
            };
            json!({
                "text": format!(" {name}"),
                "tooltip": format!("{}\n{}", st.name, human_size(st.size)),
                "class": "active",
                "alt": "active",
            })
        }
        None => json!({
            "text": "",
            "tooltip": "",
            "class": "empty",
            "alt": "empty",
        }),
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

fn human_size(bytes: u64) -> String {
    let mut size = bytes as f64;
    for unit in &["B", "KB", "MB", "GB"] {
        if size < 1024.0 {
            return if *unit == "B" {
                format!("{size} B")
            } else {
                format!("{size:.1} {unit}")
            };
        }
        size /= 1024.0;
    }
    format!("{size:.1} TB")
}
