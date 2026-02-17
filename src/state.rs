use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct FileLock {
    _file: File,
}

impl FileLock {
    fn acquire(state_file: &Path) -> Result<Self> {
        Self::acquire_timeout(state_file, std::time::Duration::from_secs(2))
    }

    fn acquire_timeout(state_file: &Path, timeout: std::time::Duration) -> Result<Self> {
        let lock_path = state_file.with_extension("lock");
        let file = File::options()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;

        // Try non-blocking first
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret == 0 {
            return Ok(Self { _file: file });
        }

        // Retry with timeout
        let deadline = std::time::Instant::now() + timeout;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if ret == 0 {
                return Ok(Self { _file: file });
            }
            if std::time::Instant::now() >= deadline {
                anyhow::bail!("timed out acquiring lock on {}", lock_path.display());
            }
        }
    }
}

// Lock is released when _file is dropped

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub time: f64,
}

impl FileState {
    pub fn new(path: PathBuf) -> Result<Self> {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs_f64();
        Ok(Self { path, name, size, time })
    }

    pub fn is_expired(&self, dismiss_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        now - self.time > (dismiss_secs + 2) as f64
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryState {
    pub entries: Vec<FileState>,
    pub selected: usize,
}

impl HistoryState {
    pub fn current(&self) -> Option<&FileState> {
        self.entries.get(self.selected)
    }

    pub fn push(&mut self, entry: FileState, max_size: usize) {
        self.entries.insert(0, entry);
        self.entries.truncate(max_size);
        self.selected = 0;
    }

    pub fn select_prev(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn select_next(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

}

fn parse_history(content: &str) -> HistoryState {
    // try new format
    if let Ok(h) = serde_json::from_str::<HistoryState>(content) {
        return h;
    }
    // backward compat: old single-FileState format
    if let Ok(fs) = serde_json::from_str::<FileState>(content) {
        return HistoryState { entries: vec![fs], selected: 0 };
    }
    HistoryState { entries: vec![], selected: 0 }
}

pub fn read_history(state_file: &Path) -> HistoryState {
    let _lock = FileLock::acquire(state_file).ok();
    let content = match std::fs::read_to_string(state_file) {
        Ok(c) => c,
        Err(_) => return HistoryState { entries: vec![], selected: 0 },
    };
    parse_history(&content)
}

/// Atomically read, modify, and write history under a single lock.
/// Use this when you need to read-then-write to avoid races.
pub fn with_history<F>(state_file: &Path, f: F) -> Result<()>
where
    F: FnOnce(&mut HistoryState),
{
    let _lock = FileLock::acquire(state_file)?;
    let content = std::fs::read_to_string(state_file).unwrap_or_default();
    let mut history = parse_history(&content);
    f(&mut history);
    let json = serde_json::to_string(&history)?;
    std::fs::write(state_file, json)?;
    Ok(())
}
