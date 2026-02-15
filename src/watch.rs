use crate::config::Config;
use crate::state::{write_state, FileState};
use anyhow::Result;
use inotify::{EventMask, Inotify, WatchMask};
use std::collections::{HashMap, VecDeque};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SEEN: usize = 1000;
const SEEN_TTL: u64 = 3600;

struct SeenCache {
    entries: VecDeque<(String, u64)>,
}

impl SeenCache {
    fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_SEEN),
        }
    }

    fn insert(&mut self, path: String) {
        let now = now_secs();
        // evict expired
        while let Some((_, ts)) = self.entries.front() {
            if now - ts > SEEN_TTL {
                self.entries.pop_front();
            } else {
                break;
            }
        }
        // evict oldest if full
        if self.entries.len() >= MAX_SEEN {
            self.entries.pop_front();
        }
        self.entries.push_back((path, now));
    }

    fn contains(&self, path: &str) -> bool {
        self.entries.iter().any(|(p, _)| p == path)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn signal_waybar(sig: u8) {
    let _ = Command::new("pkill")
        .arg(format!("-RTMIN+{sig}"))
        .arg("waybar")
        .output();
}

fn clear_state(cfg: &Config) {
    let _ = std::fs::remove_file(Config::state_file());
    signal_waybar(cfg.signal_number);
}

pub fn run(cfg: &Config) -> Result<()> {
    let state_file = Config::state_file();
    let pid_file = Config::pid_file();

    std::fs::write(&pid_file, std::process::id().to_string())?;

    // cleanup on ctrl-c / SIGTERM
    let sig_num = cfg.signal_number;
    ctrlc::set_handler(move || {
        let _ = std::fs::remove_file(Config::state_file());
        let _ = std::fs::remove_file(Config::pid_file());
        signal_waybar(sig_num);
        std::process::exit(0);
    })?;

    let mut inotify = Inotify::init()?;
    let mut wd_to_dir: HashMap<i32, PathBuf> = HashMap::new();

    for dir in &cfg.watch_dirs {
        let path = PathBuf::from(shellexpand::tilde(dir).as_ref());
        if path.is_dir() {
            let wd = inotify.watches().add(
                &path,
                WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO,
            )?;
            wd_to_dir.insert(wd.get_watch_descriptor_id(), path.clone());
            eprintln!("watching {}", path.display());
        }
    }

    let mut seen = SeenCache::new();
    let mut dismiss_at: Option<u64> = None;
    let mut buf = [0u8; 4096];

    // use poll(2) so we can wake up for dismiss timeout
    let inotify_fd = inotify.as_raw_fd();

    loop {
        // check dismiss
        if let Some(at) = dismiss_at {
            if now_secs() >= at {
                dismiss_at = None;
                clear_state(cfg);
            }
        }

        // poll with 1s timeout so we can check dismiss_at
        let mut pfd = libc::pollfd {
            fd: inotify_fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pfd as *mut _, 1, 1000) };
        if ret <= 0 {
            continue;
        }

        let events = inotify.read_events(&mut buf)?;
        for event in events {
            if !event.mask.contains(EventMask::CLOSE_WRITE)
                && !event.mask.contains(EventMask::MOVED_TO)
            {
                continue;
            }
            let Some(name) = event.name else { continue };
            let name_str = name.to_string_lossy();

            if name_str.starts_with('.') {
                continue;
            }
            if cfg
                .ignore_suffixes
                .iter()
                .any(|s| name_str.ends_with(s.as_str()))
            {
                continue;
            }

            let dir = match wd_to_dir.get(&event.wd.get_watch_descriptor_id()) {
                Some(d) => d,
                None => continue,
            };
            let path = dir.join(&*name_str);

            if !path.is_file() {
                continue;
            }
            let path_str = path.to_string_lossy().into_owned();
            if seen.contains(&path_str) {
                continue;
            }

            seen.insert(path_str);

            if let Ok(st) = FileState::new(path.clone()) {
                let _ = write_state(&state_file, &st);
                signal_waybar(cfg.signal_number);
                dismiss_at = Some(now_secs() + cfg.dismiss_seconds);
                eprintln!("new: {}", path.display());
            }
        }
    }
}
