# file-preview

Watches directories for new files and shows a transient widget in [Waybar](https://github.com/Alexays/Waybar).
Click to copy the file path, or drag-and-drop it straight into another app.

![demo](demo.gif)

## Dependencies

**Arch Linux:**

```sh
sudo pacman -S gtk4 gtk4-layer-shell wl-clipboard
```

**Fedora:**

```sh
sudo dnf install gtk4-devel gtk4-layer-shell-devel wl-clipboard
```

You also need a [Rust toolchain](https://rustup.rs/) and **Hyprland** (uses `hyprctl cursorpos` for overlay placement).

## Install

**From source:**

```sh
git clone https://github.com/areofyl/file-preview-daemon
cd file-preview-daemon
cargo build --release
cp target/release/file-preview ~/.local/bin/
```

### Autostart

Add to your Hyprland config (`~/.config/hypr/hyprland.conf`):

```
exec-once = file-preview watch
```

### Waybar module

Add to your Waybar config (`~/.config/waybar/config.jsonc`):

```jsonc
"custom/file-preview": {
    "exec": "file-preview status",
    "return-type": "json",
    "interval": 1,
    "signal": 8,
    "on-click": "file-preview drag",
    "on-click-right": "file-preview copy"
}
```

Then add `"custom/file-preview"` to your bar layout (e.g. `modules-right`).
A complete snippet is in [`waybar-module.jsonc`](waybar-module.jsonc).

### Waybar styling

Add to your Waybar CSS (`~/.config/waybar/style.css`):

```css
#custom-file-preview {
    padding: 0 8px;
    color: #cdd6f4;
}

#custom-file-preview.active {
    color: #a6e3a1;
}

#custom-file-preview.empty {
    padding: 0;
}
```

## Config

Optional. Copy `config.example.toml` to `~/.config/file-preview/config.toml`
and edit to taste. Everything has sane defaults.

```toml
# directories to watch for new files
watch_dirs = ["~/Pictures/Screenshots", "~/Downloads"]

# RTMIN+N signal to poke waybar
signal_number = 8

# auto-dismiss after N seconds
dismiss_seconds = 10

# skip partial downloads etc.
ignore_suffixes = [".part", ".crdownload", ".tmp"]

# waybar bar height in px (for drag overlay placement)
bar_height = 57
```

## Commands

```
file-preview watch    # run the inotify watcher (long-running)
file-preview status   # JSON for waybar (called by exec)
file-preview copy     # wl-copy the latest file path
file-preview drag     # drag-and-drop overlay at cursor
```

## License

[MIT](LICENSE)
