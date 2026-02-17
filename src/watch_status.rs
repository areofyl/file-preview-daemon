use crate::config::Config;
use crate::state::read_history;
use crate::util::human_size;
use anyhow::Result;
use inotify::{Inotify, WatchMask};
use serde_json::json;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::Path;

fn format_status(cfg: &Config) -> String {
    let state_file = Config::state_file();
    let history = read_history(&state_file);
    let selected = history.selected;

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
    serde_json::to_string(&output).unwrap()
}

fn emit(line: &str) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{line}");
    let _ = lock.flush();
}

pub fn run(cfg: &Config) -> Result<()> {
    let state_file = Config::state_file();

    // print initial status
    let mut last_output = format_status(cfg);
    emit(&last_output);

    // watch the state file's parent directory
    let parent = state_file.parent().unwrap_or(Path::new("/tmp"));
    let state_filename = state_file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let mut inotify = Inotify::init()?;
    inotify.watches().add(
        parent,
        WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO | WatchMask::MODIFY,
    )?;

    let inotify_fd = inotify.as_raw_fd();
    let mut buf = [0u8; 4096];

    loop {
        // poll with 1s timeout to check dismiss timer
        let mut pfd = libc::pollfd {
            fd: inotify_fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pfd as *mut _, 1, 1000) };

        if ret > 0 {
            // drain inotify events
            if let Ok(mut events) = inotify.read_events(&mut buf) {
                let relevant = events
                    .any(|e| e.name.map_or(false, |n| n.to_string_lossy() == state_filename));
                if relevant {
                    let new_output = format_status(cfg);
                    if new_output != last_output {
                        emit(&new_output);
                        last_output = new_output;
                    }
                }
            }
        } else {
            // timeout — re-check for dismiss expiry
            let new_output = format_status(cfg);
            if new_output != last_output {
                emit(&new_output);
                last_output = new_output;
            }
        }
    }
}
