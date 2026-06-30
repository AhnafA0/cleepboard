mod clipboard;
mod hotkey;
mod store;

use clipboard::Backend;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use store::{ClipItem, Settings, Store};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

pub struct AppState {
    store: Mutex<Store>,
    backend: Backend,
    // Signature of the value we last *set* ourselves, so the watcher ignores it.
    self_set: Mutex<Option<String>>,
}

fn sig_text(t: &str) -> String {
    format!("t:{}", t)
}
fn sig_image(len: usize) -> String {
    format!("i:{}", len)
}
fn sig_file(payload: &str) -> String {
    format!("f:{}", payload)
}

#[tauri::command]
fn get_history(state: State<AppState>) -> Vec<ClipItem> {
    state.store.lock().unwrap().items.clone()
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.store.lock().unwrap().settings.clone()
}

#[tauri::command]
fn set_settings(state: State<AppState>, settings: Settings) -> Settings {
    let mut store = state.store.lock().unwrap();
    store.settings = settings;
    store.save_settings();
    store.settings.clone()
}

#[tauri::command]
fn toggle_pin(state: State<AppState>, id: String) -> Vec<ClipItem> {
    let mut store = state.store.lock().unwrap();
    store.toggle_pin(&id);
    store.items.clone()
}

#[tauri::command]
fn delete_item(state: State<AppState>, id: String) -> Vec<ClipItem> {
    let mut store = state.store.lock().unwrap();
    store.delete(&id);
    store.items.clone()
}

#[tauri::command]
fn clear_history(state: State<AppState>, keep_pinned: bool) -> Vec<ClipItem> {
    let mut store = state.store.lock().unwrap();
    store.clear(keep_pinned);
    store.items.clone()
}

/// Read the full text of an item (for the detail view).
#[tauri::command]
fn get_item_text(state: State<AppState>, id: String) -> Option<String> {
    let store = state.store.lock().unwrap();
    store.find(&id).and_then(|i| i.text.clone())
}

/// Return a data URL for an image item so the frontend can render it.
#[tauri::command]
fn get_image_data_url(state: State<AppState>, id: String) -> Option<String> {
    let path = {
        let store = state.store.lock().unwrap();
        store.find(&id).and_then(|i| i.image_path.clone())?
    };
    let bytes = std::fs::read(path).ok()?;
    Some(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

#[tauri::command]
fn copy_item(app: AppHandle, state: State<AppState>, id: String, paste: bool) -> bool {
    let backend = state.backend;
    let (kind, text, image_path) = {
        let store = state.store.lock().unwrap();
        match store.find(&id) {
            Some(i) => (i.kind.clone(), i.text.clone(), i.image_path.clone()),
            None => return false,
        }
    };

    let ok = match kind.as_str() {
        "text" => {
            let t = text.unwrap_or_default();
            *state.self_set.lock().unwrap() = Some(sig_text(&t));
            clipboard::set_text(backend, &t)
        }
        "image" => match image_path.and_then(|p| std::fs::read(p).ok()) {
            Some(bytes) => {
                *state.self_set.lock().unwrap() = Some(sig_image(bytes.len()));
                clipboard::set_image(backend, &bytes)
            }
            None => false,
        },
        // File clips store the raw `text/uri-list` payload in `text`; re-copy
        // must write it back with the `text/uri-list` mime type so file
        // managers accept the paste.
        "file" => {
            let payload = text.unwrap_or_default();
            *state.self_set.lock().unwrap() = Some(sig_file(&payload));
            clipboard::set_files(backend, &payload)
        }
        _ => false,
    };

    // Hide the overlay before pasting so focus returns to the target app.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    if ok && paste {
        // Small delay so the compositor restores focus to the previous window.
        thread::sleep(Duration::from_millis(120));
        clipboard::auto_paste(backend);
    }
    ok
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn get_data_dir(state: State<AppState>) -> String {
    state.store.lock().unwrap().dir().to_string_lossy().to_string()
}

#[tauri::command]
fn set_data_dir(app: AppHandle, state: State<AppState>, new_dir: String) -> Result<Settings, String> {
    if new_dir.trim().is_empty() {
        return Err("Directory path cannot be empty".into());
    }
    let new_path = PathBuf::from(&new_dir);

    let mut store = state.store.lock().unwrap();
    let current_dir = store.dir().to_path_buf();

    if current_dir.components().eq(new_path.components()) {
        return Ok(store.settings.clone());
    }

    let new_store = store.migrate_to(&new_path)?;
    let settings = new_store.settings.clone();
    *store = new_store;
    drop(store);

    // Update bootstrap settings in default config dir so future startups find it.
    let default_dir = store::config_dir_for("cleepboard");
    let _ = fs::create_dir_all(&default_dir);
    let bootstrap = Settings {
        data_dir: new_path.to_string_lossy().to_string(),
        ..settings.clone()
    };
    if let Ok(s) = serde_json::to_string_pretty(&bootstrap) {
        let _ = fs::write(default_dir.join("settings.json"), s);
    }

    let _ = app.emit("settings-updated", settings.clone());
    Ok(settings)
}

#[tauri::command]
fn register_hotkey(binding: String) -> Result<String, String> {
    hotkey::register(&binding)
}

#[tauri::command]
fn unregister_hotkey() -> Result<String, String> {
    hotkey::unregister()
}

#[tauri::command]
fn hotkey_status() -> bool {
    hotkey::is_registered()
}

#[tauri::command]
fn paste_tool_available(state: State<AppState>) -> bool {
    match state.backend {
        Backend::Wayland => clipboard::which("wtype") || clipboard::which("ydotool"),
        Backend::X11 => clipboard::which("xdotool"),
    }
}

fn show_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        // On X11, position the overlay near the pointer (NoteHistory: "Appears
        // near cursor"). Wayland clients can't freely position their own
        // windows or read the pointer from a background process, so there we
        // rely on `center: true` from the window config instead.
        let backend = app.state::<AppState>().backend;
        if backend == Backend::X11 {
            position_near_cursor(&win);
        }
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit("overlay-shown", ());
    }
}

/// Best-effort near-cursor positioning for X11. Reads the pointer via
/// `xdotool getmouselocation` and places the window's top-left so the overlay
/// opens roughly under the cursor, clamped to the current monitor's bounds.
/// Silently does nothing if xdotool is missing or anything fails — the window
/// then keeps its configured `center: true` position.
fn position_near_cursor(win: &tauri::WebviewWindow) {
    use tauri::PhysicalPosition;

    if !clipboard::which("xdotool") {
        return;
    }
    let out = match clipboard::run_capture("xdotool", &["getmouselocation"]) {
        Some(s) => s,
        None => return,
    };
    let (mut x, mut y) = match parse_mouse_location(&out) {
        Some(p) => p,
        None => return,
    };

    let size = win.outer_size().unwrap_or_default();
    let (w, h) = (size.width as i32, size.height as i32);

    // Center the window on the cursor horizontally; sit just above it so the
    // overlay doesn't cover what the user is about to paste into.
    x -= w / 2;
    y -= h + 12;

    // Clamp to the monitor the cursor is on so we never land off-screen.
    if let Ok(Some(monitor)) = win.current_monitor() {
        let pos = monitor.position();
        let mon = monitor.size();
        let min_x = pos.x;
        let min_y = pos.y;
        let max_x = pos.x + mon.width as i32 - w;
        let max_y = pos.y + mon.height as i32 - h;
        if max_x > min_x {
            x = x.clamp(min_x, max_x);
        } else {
            x = min_x;
        }
        if max_y > min_y {
            y = y.clamp(min_y, max_y);
        } else {
            y = min_y;
        }
    } else {
        if x < 0 { x = 0; }
        if y < 0 { y = 0; }
    }

    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// Parse `x:N y:N screen:N window:N` from `xdotool getmouselocation`.
fn parse_mouse_location(s: &str) -> Option<(i32, i32)> {
    let mut x = None;
    let mut y = None;
    for part in s.split_whitespace() {
        if let Some(rest) = part.strip_prefix("x:") {
            x = rest.parse::<i32>().ok();
        } else if let Some(rest) = part.strip_prefix("y:") {
            y = rest.parse::<i32>().ok();
        }
    }
    Some((x?, y?))
}

fn toggle_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => {
                let _ = win.hide();
            }
            _ => show_overlay(app),
        }
    }
}

/// Background thread: poll the clipboard and append new content to history.
fn spawn_watcher(app: AppHandle) {
    thread::spawn(move || {
        let state = app.state::<AppState>();
        let backend = state.backend;
        let mut last_sig: Option<String> = None;

        loop {
            let poll_ms = {
                let store = state.store.lock().unwrap();
                store.settings.poll_ms.max(150)
            };
            thread::sleep(Duration::from_millis(poll_ms));

            let content = clipboard::read(backend);
            let sig = match &content {
                clipboard::ClipContent::Text(t) => Some(sig_text(t)),
                clipboard::ClipContent::Image(b) => Some(sig_image(b.len())),
                clipboard::ClipContent::Files(uris) => Some(sig_file(&uris.join("\n"))),
                clipboard::ClipContent::Empty => None,
            };
            let sig = match sig {
                Some(s) => s,
                None => continue,
            };

            // Skip if unchanged since last poll.
            if last_sig.as_deref() == Some(sig.as_str()) {
                continue;
            }
            // Skip values we set ourselves.
            if state.self_set.lock().unwrap().as_deref() == Some(sig.as_str()) {
                last_sig = Some(sig);
                continue;
            }
            last_sig = Some(sig);

            // Snapshot the focused window *before* we touch the clipboard so
            // the source-app metadata reflects the app that owned the
            // selection at copy time. X11-only; None on Wayland.
            let source_app = clipboard::active_window_name(backend);

            let added = {
                let mut store = state.store.lock().unwrap();
                match content {
                    clipboard::ClipContent::Text(t) => store.add_text(t, source_app.clone()),
                    clipboard::ClipContent::Image(b) => store.add_image(&b, source_app.clone()),
                    clipboard::ClipContent::Files(uris) => store.add_file(uris, source_app.clone()),
                    clipboard::ClipContent::Empty => false,
                }
            };
            if added {
                let items = state.store.lock().unwrap().items.clone();
                let _ = app.emit("history-updated", items);
            }
        }
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "Open Cleepboard", true, None::<&str>)?;
    let clear_i = MenuItem::with_id(app, "clear", "Clear history", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &clear_i, &quit_i])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Cleepboard")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_overlay(app),
            "clear" => {
                let state = app.state::<AppState>();
                let items = {
                    let mut store = state.store.lock().unwrap();
                    store.clear(true);
                    store.items.clone()
                };
                let _ = app.emit("history-updated", items);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                toggle_overlay(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's DMABUF renderer (default since 2.42) does not do correct
    // damage tracking for transparent webviews under GNOME/Mutter on Wayland.
    // The result is constant full-surface repaints — the screen appears to
    // continuously "refresh" and the dock's active-window indicator flickers.
    // Disabling the DMABUF renderer restores normal damage-based compositing
    // while keeping the transparent rounded-overlay design intact.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let backend = clipboard::detect_backend();
    if backend == Backend::Wayland {
        // Best-effort; only relevant on Wayland and harmless to retry every
        // launch. See doc comment on the function for why this is needed.
        clipboard::suppress_wl_clipboard_notifications();
    }
    let default_dir = store::config_dir_for("cleepboard");
    let _ = fs::create_dir_all(&default_dir);

    let bootstrap_settings: Settings = fs::read_to_string(default_dir.join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let store_dir = if bootstrap_settings.data_dir.is_empty() {
        default_dir.clone()
    } else {
        let custom = PathBuf::from(&bootstrap_settings.data_dir);
        if custom.join("settings.json").exists() {
            custom
        } else {
            default_dir.clone()
        }
    };

    let store = Store::load(store_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // A second launch (e.g. from the global hotkey: `cleepboard --toggle`)
            // routes here. Toggle the overlay instead of opening a new window.
            if argv.iter().any(|a| a == "--toggle") {
                toggle_overlay(app);
            } else {
                show_overlay(app);
            }
        }))
        .manage(AppState {
            store: Mutex::new(store),
            backend,
            self_set: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            get_settings,
            set_settings,
            toggle_pin,
            delete_item,
            clear_history,
            get_item_text,
            get_image_data_url,
            copy_item,
            hide_window,
            register_hotkey,
            unregister_hotkey,
            hotkey_status,
            paste_tool_available,
            get_data_dir,
            set_data_dir,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;
            spawn_watcher(handle.clone());

            // If launched with --toggle on first start, still show the window.
            let args: Vec<String> = std::env::args().collect();
            let start_hidden = !args.iter().any(|a| a == "--show");
            if let Some(win) = app.get_webview_window("main") {
                if start_hidden {
                    let _ = win.hide();
                } else {
                    show_overlay(&handle);
                }
                let h = handle.clone();
                win.on_window_event(move |event| {
                    // Hide instead of close; the app lives in the tray.
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = h.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                    // NOTE: We deliberately do NOT auto-hide on focus loss.
                    // On GNOME/Wayland this undecorated, always-on-top overlay
                    // cannot reliably hold keyboard focus — it receives a
                    // Focused(true) immediately followed by Focused(false) right
                    // after being shown, which would instantly hide it and make
                    // the app look like it crashed. The overlay is dismissed via
                    // Esc (handled in the frontend -> hide_window), by selecting
                    // an item, or by toggling from the tray.
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running cleepboard");
}

// Minimal base64 encoder (avoids pulling an extra crate).
fn base64_encode(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
