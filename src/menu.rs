use crate::config::Config;
use crate::state::read_history;
use anyhow::Result;
use gtk4::gdk;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::process::Command;
use std::time::Duration;

const THUMB_MAX: i32 = 150;
const MENU_W: i32 = 220;
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "tiff"];

fn is_image(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn cursor_pos() -> Option<(i32, i32)> {
    let out = Command::new("hyprctl").arg("cursorpos").output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let parts: Vec<&str> = text.trim().split(',').collect();
    if parts.len() >= 2 {
        Some((parts[0].trim().parse().ok()?, parts[1].trim().parse().ok()?))
    } else {
        None
    }
}

fn find_monitor_at(gx: i32, gy: i32) -> Option<(String, i32, i32)> {
    let out = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let monitors: Vec<serde_json::Value> = serde_json::from_str(&text).ok()?;
    for m in &monitors {
        let name = m["name"].as_str()?;
        let x = m["x"].as_i64()? as i32;
        let y = m["y"].as_i64()? as i32;
        let w = m["width"].as_i64()? as i32;
        let h = m["height"].as_i64()? as i32;
        if gx >= x && gx < x + w && gy >= y && gy < y + h {
            return Some((name.to_string(), x, y));
        }
    }
    None
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

fn menu_lock_path() -> std::path::PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(runtime).join("glance-menu.lock")
}

fn menu_pos_path() -> std::path::PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(runtime).join("glance-menu-pos")
}

fn read_saved_pos(path: &std::path::Path) -> Option<(i32, i32)> {
    let content = std::fs::read_to_string(path).ok()?;
    let parts: Vec<&str> = content.trim().split(',').collect();
    if parts.len() == 2 {
        Some((parts[0].parse().ok()?, parts[1].parse().ok()?))
    } else {
        None
    }
}

fn save_pos(path: &std::path::Path, x: i32, y: i32) {
    let _ = std::fs::write(path, format!("{x},{y}"));
}

fn menu_pid_path() -> std::path::PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(runtime).join("glance-menu.pid")
}

fn kill_existing_menu() {
    if let Ok(pid_str) = std::fs::read_to_string(menu_pid_path()) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            unsafe { libc::kill(pid, libc::SIGTERM); }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let _ = std::fs::remove_file(menu_pid_path());
    let _ = std::fs::remove_file(menu_lock_path());
}

fn write_menu_pid() {
    let _ = std::fs::write(menu_pid_path(), std::process::id().to_string());
}

pub fn run(cfg: &Config) -> Result<()> {
    let history = read_history(&Config::state_file());
    let Some(st) = history.current().filter(|e| !e.is_expired(cfg.dismiss_seconds)) else {
        return Ok(());
    };
    if !st.path.exists() {
        return Ok(());
    }

    let filepath = st.path.clone();
    let filesize = st.size;
    let bar_height = cfg.bar_height;

    // use saved module position if available, otherwise capture from cursor
    let pos_file = menu_pos_path();
    let (cursor_x, cursor_y) = if let Some((x, y)) = read_saved_pos(&pos_file) {
        (x, y)
    } else {
        let pos = cursor_pos().unwrap_or((800, 0));
        save_pos(&pos_file, pos.0, pos.1);
        pos
    };
    let monitor_info = find_monitor_at(cursor_x, cursor_y);

    // kill any existing menu instance
    kill_existing_menu();
    write_menu_pid();
    let _ = std::fs::write(menu_lock_path(), "");

    let app = gtk4::Application::builder()
        .application_id(&format!("dev.glance.menu.{}", std::process::id()))
        .build();

    app.connect_activate(move |app| {
        let win = gtk4::ApplicationWindow::new(app);

        win.init_layer_shell();
        win.set_layer(Layer::Overlay);
        win.set_anchor(Edge::Top, true);
        win.set_anchor(Edge::Left, true);

        // position below waybar, centered on cursor X (which is on the module)
        if let Some((ref mon_name, mon_x, _)) = monitor_info {
            let display = gdk::Display::default().unwrap();
            let monitors = display.monitors();
            for i in 0..monitors.n_items() {
                if let Some(obj) = monitors.item(i) {
                    let mon = obj.downcast::<gdk::Monitor>().unwrap();
                    if mon.connector().map(|c| c.as_str() == mon_name).unwrap_or(false) {
                        win.set_monitor(Some(&mon));
                        break;
                    }
                }
            }
            let local_x = cursor_x - mon_x;
            win.set_margin(Edge::Left, (local_x - MENU_W / 2).max(0));
        } else {
            win.set_margin(Edge::Left, (cursor_x - MENU_W / 2).max(0));
        }
        win.set_margin(Edge::Top, bar_height);
        win.set_exclusive_zone(-1);
        win.set_namespace(Some("glance-menu"));
        win.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);

        let css = gtk4::CssProvider::new();
        #[allow(deprecated)]
        css.load_from_data(
            "window { background: transparent; } \
             .menu { background: rgba(30,30,46,0.95); border-radius: 12px; padding: 10px; } \
             .menu-size { color: #a6adc8; font-size: 11px; margin-top: 2px; } \
             .menu-actions { margin-top: 8px; } \
             .menu-action { background: rgba(255,255,255,0.08); color: #cdd6f4; \
               border: none; border-radius: 8px; padding: 6px 14px; min-height: 0; min-width: 0; } \
             .menu-action:hover { background: rgba(255,255,255,0.15); } \
             .menu-close { background: none; border: none; color: #a6adc8; \
               min-height: 0; min-width: 0; padding: 2px 6px; } \
             .menu-close:hover { color: #f38ba8; }",
        );
        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        // thumbnail or icon
        if is_image(&filepath) {
            if let Ok(pixbuf) = Pixbuf::from_file(&filepath) {
                let (w, h) = (pixbuf.width(), pixbuf.height());
                let scale = (THUMB_MAX as f64) / (w.max(h) as f64);
                let new_w = ((w as f64 * scale) as i32).max(1);
                let new_h = ((h as f64 * scale) as i32).max(1);
                if let Some(scaled) = pixbuf.scale_simple(
                    new_w,
                    new_h,
                    gtk4::gdk_pixbuf::InterpType::Bilinear,
                ) {
                    let texture = gdk::Texture::for_pixbuf(&scaled);
                    let picture = gtk4::Picture::for_paintable(&texture);
                    picture.set_size_request(new_w, new_h);
                    container.append(&picture);
                }
            }
        } else {
            let icon = gtk4::Image::from_icon_name("text-x-generic");
            icon.set_pixel_size(48);
            container.append(&icon);
        }

        // file size
        let size_label = gtk4::Label::new(Some(&human_size(filesize)));
        size_label.add_css_class("menu-size");
        container.append(&size_label);

        // action buttons
        let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        actions.add_css_class("menu-actions");
        actions.set_halign(gtk4::Align::Center);

        // Drag — uses a DragSource so you grab and drag directly from this button
        let btn_drag = gtk4::Label::new(Some("Drag"));
        btn_drag.add_css_class("menu-action");
        btn_drag.set_size_request(60, -1);
        let ds = gtk4::DragSource::new();
        ds.set_actions(gdk::DragAction::COPY);
        let file = gio::File::for_path(&filepath);
        let uri = format!("{}\r\n", file.uri());
        ds.connect_prepare(move |_, _, _| {
            Some(gdk::ContentProvider::for_bytes(
                "text/uri-list",
                &glib::Bytes::from(uri.as_bytes()),
            ))
        });
        ds.connect_drag_end(move |_, _, _| {
            glib::timeout_add_local_once(Duration::from_millis(200), move || {
                std::process::exit(0);
            });
        });
        btn_drag.add_controller(ds);
        actions.append(&btn_drag);

        // Open
        let btn_open = gtk4::Button::with_label("Open");
        btn_open.add_css_class("menu-action");
        let p = filepath.clone();
        btn_open.connect_clicked(move |_| {
            let _ = Command::new("xdg-open").arg(&p).spawn();
            std::process::exit(0);
        });
        actions.append(&btn_open);

        // Copy
        let btn_copy = gtk4::Button::with_label("Copy");
        btn_copy.add_css_class("menu-action");
        let p = filepath.clone();
        btn_copy.connect_clicked(move |_| {
            let _ = Command::new("wl-copy")
                .arg(p.to_string_lossy().as_ref())
                .output();
            std::process::exit(0);
        });
        actions.append(&btn_copy);

        container.append(&actions);

        // close button at top-right
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let spacer = gtk4::Label::new(None);
        spacer.set_hexpand(true);
        header.append(&spacer);
        let btn_close = gtk4::Button::with_label("\u{2715}");
        btn_close.add_css_class("menu-close");
        btn_close.connect_clicked(move |_| {
            std::process::exit(0);
        });
        header.append(&btn_close);

        let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        outer.add_css_class("menu");
        outer.append(&header);
        outer.append(&container);

        win.set_child(Some(&outer));
        win.present();

        // escape to dismiss
        let key_ctl = gtk4::EventControllerKey::new();
        key_ctl.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                std::process::exit(0);
            } else {
                glib::Propagation::Proceed
            }
        });
        win.add_controller(key_ctl);

        // auto-dismiss 8s
        glib::timeout_add_local_once(Duration::from_secs(8), move || {
            std::process::exit(0);
        });
    });

    app.run_with_args::<&str>(&[]);
    let _ = std::fs::remove_file(menu_pid_path());
    let _ = std::fs::remove_file(menu_lock_path());
    Ok(())
}
