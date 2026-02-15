use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn read_history(state_file: &Path) -> HistoryState {
    let content = match std::fs::read_to_string(state_file) {
        Ok(c) => c,
        Err(_) => return HistoryState { entries: vec![], selected: 0 },
    };
    // try new format
    if let Ok(h) = serde_json::from_str::<HistoryState>(&content) {
        return h;
    }
    // backward compat: old single-FileState format
    if let Ok(fs) = serde_json::from_str::<FileState>(&content) {
        return HistoryState { entries: vec![fs], selected: 0 };
    }
    HistoryState { entries: vec![], selected: 0 }
}

pub fn write_history(state_file: &Path, state: &HistoryState) -> Result<()> {
    let json = serde_json::to_string(state)?;
    std::fs::write(state_file, json)?;
    Ok(())
}
