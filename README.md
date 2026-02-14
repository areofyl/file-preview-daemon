# file-preview

Watches directories for new files and shows a transient widget in [Waybar](https://github.com/Alexays/Waybar).
Clicking the widget copies the file path to clipboard via `wl-copy`.

<!-- ![demo](demo.gif) -->

## Requirements

- Python 3.11+ (`tomllib`; or 3.10 with `pip install tomli`)
- `pip install inotify`
- `wl-clipboard`

## Install

```sh
git clone https://github.com/youruser/file-preview
# start the daemon (add to your compositor autostart)
python3 file-preview/file-preview-daemon.py watch &
```

Add the Waybar module to `~/.config/waybar/config.jsonc`:

```jsonc
"custom/file-preview": {
    "exec": "python3 /path/to/file-preview-daemon.py status",
    "return-type": "json",
    "interval": 1,
    "signal": 8,
    "on-click": "python3 /path/to/file-preview-daemon.py copy"
}
```

Then add `"custom/file-preview"` to your bar layout. A complete snippet
is in [`waybar-module.jsonc`](waybar-module.jsonc).

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
```

## Commands

```
file-preview-daemon.py watch    # run the inotify watcher (long-running)
file-preview-daemon.py status   # JSON for waybar (called by exec)
file-preview-daemon.py copy     # wl-copy the latest file path
```

## License

MIT
