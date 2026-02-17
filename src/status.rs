use crate::config::Config;
use crate::state::read_history;
use crate::util::human_size;
use anyhow::Result;
use serde_json::json;

pub fn run(cfg: &Config, index_override: Option<usize>) -> Result<()> {
    let state_file = Config::state_file();
    let history = read_history(&state_file);

    // use override if provided, otherwise use persisted selection
    let selected = index_override.unwrap_or(history.selected);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let recently_scrolled = history.last_scroll > 0.0
        && (now - history.last_scroll) < cfg.dismiss_seconds as f64;
    let manually_scrolled = selected != 0;

    let current = history
        .entries
        .get(selected)
        .filter(|e| manually_scrolled || recently_scrolled || !e.is_expired(cfg.dismiss_seconds));

    let active_count = history.entries.len();

    let output = match current {
        Some(st) => {
            let name = if st.name.len() > 18 {
                format!("{}\u{2026}", &st.name[..15])
            } else {
                st.name.clone()
            };
            let count_suffix = if active_count > 1 {
                format!(" ({}/{})", selected + 1, active_count)
            } else {
                String::new()
            };
            let tooltip_lines: Vec<String> = history
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let marker = if i == selected { "▸" } else { " " };
                    format!("{marker} {} ({})", e.name, human_size(e.size))
                })
                .collect();
            json!({
                "text": format!(" {name}{count_suffix}"),
                "tooltip": tooltip_lines.join("\n"),
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

