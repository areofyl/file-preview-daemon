#!/usr/bin/env python3
"""file-preview — inotify watcher + waybar widget for new files."""

import os
import sys

# ── config ────────────────────────────────────────────────────────

DEFAULTS = {
    "watch_dirs": ["~/Pictures/Screenshots", "~/Downloads"],
    "signal_number": 8,
    "dismiss_seconds": 10,
    "ignore_suffixes": [".part", ".crdownload", ".tmp"],
}

CONFIG_PATH = os.path.expanduser("~/.config/file-preview/config.toml")


def load_config():
    cfg = dict(DEFAULTS)
    if os.path.exists(CONFIG_PATH):
        try:
            try:
                import tomllib
            except ModuleNotFoundError:
                import tomli as tomllib
            with open(CONFIG_PATH, "rb") as f:
                user = tomllib.load(f)
            for k in DEFAULTS:
                if k in user:
                    cfg[k] = user[k]
        except Exception as e:
            print(f"file-preview: config error: {e}", file=sys.stderr)
    cfg["watch_dirs"] = [os.path.expanduser(d) for d in cfg["watch_dirs"]]
    return cfg


CFG = load_config()

_runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
STATE_FILE = os.path.join(_runtime, "file-preview-latest.json")
PID_FILE = os.path.join(_runtime, "file-preview.pid")

# ── status / copy (fast path, no inotify import) ─────────────────

if len(sys.argv) >= 2 and sys.argv[1] in ("status", "copy"):
    import json
    import subprocess
    import time

    def _human_size(n):
        for u in ("B", "KB", "MB", "GB"):
            if n < 1024:
                return f"{n:.1f} {u}" if u != "B" else f"{n} {u}"
            n /= 1024
        return f"{n:.1f} TB"

    def _read_state():
        try:
            with open(STATE_FILE) as f:
                st = json.load(f)
            if time.time() - st["time"] > CFG["dismiss_seconds"] + 2:
                return None
            return st
        except (FileNotFoundError, json.JSONDecodeError, KeyError):
            return None

    if sys.argv[1] == "status":
        st = _read_state()
        if st is None:
            print(json.dumps({"text": "", "tooltip": "", "class": "empty", "alt": "empty"}))
        else:
            name = st["name"]
            if len(name) > 18:
                name = name[:15] + "\u2026"
            print(json.dumps({
                "text": f" {name}",
                "tooltip": f"{st['name']}\n{_human_size(st['size'])}",
                "class": "active",
                "alt": "active",
            }))

    elif sys.argv[1] == "copy":
        st = _read_state()
        if st and os.path.exists(st["path"]):
            subprocess.run(["wl-copy", st["path"]], capture_output=True)

    sys.exit(0)

# ── watch ─────────────────────────────────────────────────────────

import json
import signal
import subprocess
import time

try:
    import inotify.adapters
except ImportError:
    sys.exit("file-preview: missing 'inotify' — pip install inotify")


def signal_waybar():
    subprocess.run(["pkill", f"-RTMIN+{CFG['signal_number']}", "waybar"],
                   capture_output=True)


def write_state(path):
    try:
        sz = os.path.getsize(path)
    except OSError:
        sz = 0
    with open(STATE_FILE, "w") as f:
        json.dump({"path": path, "name": os.path.basename(path),
                    "size": sz, "time": time.time()}, f)


def clear_state():
    try:
        os.unlink(STATE_FILE)
    except FileNotFoundError:
        pass
    signal_waybar()


def main():
    if len(sys.argv) < 2 or sys.argv[1] != "watch":
        print(f"usage: {sys.argv[0]} {{watch|status|copy}}")
        sys.exit(1)

    with open(PID_FILE, "w") as f:
        f.write(str(os.getpid()))

    def _shutdown(signum, frame):
        clear_state()
        try:
            os.unlink(PID_FILE)
        except FileNotFoundError:
            pass
        sys.exit(0)

    signal.signal(signal.SIGTERM, _shutdown)
    signal.signal(signal.SIGINT, _shutdown)

    ino = inotify.adapters.Inotify()
    for d in CFG["watch_dirs"]:
        if os.path.isdir(d):
            ino.add_watch(d)
            print(f"watching {d}")

    seen = set()
    dismiss_at = 0

    for ev in ino.event_gen(yield_nones=True):
        if dismiss_at and time.time() > dismiss_at:
            dismiss_at = 0
            clear_state()

        if ev is None:
            continue

        _, types, watch_path, filename = ev
        if "IN_CLOSE_WRITE" not in types and "IN_MOVED_TO" not in types:
            continue
        if not filename or filename.startswith("."):
            continue
        if any(filename.endswith(s) for s in CFG["ignore_suffixes"]):
            continue

        path = os.path.join(watch_path, filename)
        if not os.path.isfile(path) or path in seen:
            continue

        seen.add(path)
        write_state(path)
        signal_waybar()
        dismiss_at = time.time() + CFG["dismiss_seconds"]
        print(f"new: {path}")


if __name__ == "__main__":
    main()
