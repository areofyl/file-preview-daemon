use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

fn default_watch_dirs() -> Vec<String> {
    vec![
        "~/Pictures/Screenshots".into(),
        "~/Downloads".into(),
    ]
}
fn default_signal_number() -> u8 {
    8
}
fn default_dismiss_seconds() -> u64 {
    10
}
fn default_ignore_suffixes() -> Vec<String> {
    vec![".part".into(), ".crdownload".into(), ".tmp".into()]
}
fn default_bar_height() -> i32 {
    57
}
fn default_history_size() -> usize {
    5
}
fn default_editor() -> String {
    "swappy".into()
}
fn default_actions() -> Vec<String> {
    vec!["drag".into(), "open".into(), "edit".into(), "copy".into()]
}
fn default_menu_dismiss_seconds() -> u64 {
    8
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuStyle {
    #[serde(default = "MenuStyle::default_background")]
    pub background: String,
    #[serde(default = "MenuStyle::default_text_color")]
    pub text_color: String,
    #[serde(default = "MenuStyle::default_secondary_color")]
    pub secondary_color: String,
    #[serde(default = "MenuStyle::default_button_background")]
    pub button_background: String,
    #[serde(default = "MenuStyle::default_button_hover")]
    pub button_hover: String,
    #[serde(default = "MenuStyle::default_border_radius")]
    pub border_radius: i32,
}

impl MenuStyle {
    fn default_background() -> String { "rgba(30,30,46,0.95)".into() }
    fn default_text_color() -> String { "#cdd6f4".into() }
    fn default_secondary_color() -> String { "#a6adc8".into() }
    fn default_button_background() -> String { "rgba(255,255,255,0.08)".into() }
    fn default_button_hover() -> String { "rgba(255,255,255,0.15)".into() }
    fn default_border_radius() -> i32 { 12 }
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            background: Self::default_background(),
            text_color: Self::default_text_color(),
            secondary_color: Self::default_secondary_color(),
            button_background: Self::default_button_background(),
            button_hover: Self::default_button_hover(),
            border_radius: Self::default_border_radius(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_watch_dirs")]
    pub watch_dirs: Vec<String>,
    #[serde(default = "default_signal_number")]
    pub signal_number: u8,
    #[serde(default = "default_dismiss_seconds")]
    pub dismiss_seconds: u64,
    #[serde(default = "default_ignore_suffixes")]
    pub ignore_suffixes: Vec<String>,
    #[serde(default = "default_bar_height")]
    pub bar_height: i32,
    #[serde(default = "default_history_size")]
    pub history_size: usize,
    #[serde(default = "default_editor")]
    pub editor: String,
    #[serde(default = "default_actions")]
    pub actions: Vec<String>,
    #[serde(default = "default_menu_dismiss_seconds")]
    pub menu_dismiss_seconds: u64,
    #[serde(default)]
    pub menu_style: MenuStyle,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_dirs: default_watch_dirs(),
            signal_number: default_signal_number(),
            dismiss_seconds: default_dismiss_seconds(),
            ignore_suffixes: default_ignore_suffixes(),
            bar_height: default_bar_height(),
            history_size: default_history_size(),
            editor: default_editor(),
            actions: default_actions(),
            menu_dismiss_seconds: default_menu_dismiss_seconds(),
            menu_style: MenuStyle::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = config_path();
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&config_path)?;
        let mut cfg: Config = toml::from_str(&content)?;
        cfg.watch_dirs = cfg
            .watch_dirs
            .into_iter()
            .map(|d| shellexpand::tilde(&d).into_owned())
            .collect();
        Ok(cfg)
    }

    pub fn has_action(&self, name: &str) -> bool {
        self.actions.iter().any(|a| a == name)
    }

    pub fn state_file() -> PathBuf {
        runtime_dir().join("glance-latest.json")
    }

    pub fn pid_file() -> PathBuf {
        runtime_dir().join("glance.pid")
    }
}

fn runtime_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()),
    )
}

fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
                .join(".config")
        });
    base.join("glance/config.toml")
}
