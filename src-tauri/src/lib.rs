mod clipboard;
mod deps;
mod hotkey;
mod store;

use clipboard::Backend;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use store::{ClipItem, Settings, Store};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};

pub struct AppState {
    store: Mutex<Store>,
    backend: Backend,
    // Signature of the value we last *set* ourselves, so the watcher ignores
    // it. Consumed by the watcher the first time it is observed (whether the
    // clipboard matches it or has moved on) — otherwise an external copy of
    // identical content would be silently dropped forever.
    self_set: Mutex<Option<String>>,
}

#[tauri::command]
fn get_history(state: State<AppState>) -> Vec<ClipItem> {
    state.store.lock().unwrap().summaries()
}

/// Case-insensitive search over preview + the leading slice of each text
/// payload (the list payload carries no `text` — see `Store::summaries`).
#[tauri::command]
fn search_history(state: State<AppState>, query: String) -> Vec<ClipItem> {
    state.store.lock().unwrap().search(&query)
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
    store.summaries()
}

#[tauri::command]
fn delete_item(state: State<AppState>, id: String) -> Vec<ClipItem> {
    let mut store = state.store.lock().unwrap();
    store.delete(&id);
    store.summaries()
}

#[tauri::command]
fn clear_history(state: State<AppState>, keep_pinned: bool) -> Vec<ClipItem> {
    let mut store = state.store.lock().unwrap();
    store.clear(keep_pinned);
    store.summaries()
}

/// Read the full text of an item (for the detail view).
#[tauri::command]
fn get_item_text(state: State<AppState>, id: String) -> Option<String> {
    let store = state.store.lock().unwrap();
    store.find(&id).and_then(|i| i.text.clone())
}

/// Return a data URL for an image item's thumbnail — bounded (~96px) so the
/// list doesn't pull every full-size PNG over IPC on each render.
#[tauri::command]
fn get_image_thumb(state: State<AppState>, id: String) -> Option<String> {
    let path = {
        let store = state.store.lock().unwrap();
        let item = store.find(&id)?;
        store.ensure_thumb(item)?
    };
    let bytes = std::fs::read(path).ok()?;
    Some(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

/// Return a data URL for an image item's full-size PNG (detail view only).
#[tauri::command]
fn get_image_data_url(state: State<AppState>, id: String) -> Option<String> {
    let path = {
        let store = state.store.lock().unwrap();
        let item = store.find(&id)?;
        store.image_file(item)?
    };
    let bytes = std::fs::read(path).ok()?;
    Some(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

#[tauri::command]
fn copy_item(app: AppHandle, state: State<AppState>, id: String, paste: bool) -> bool {
    let backend = state.backend;
    let (kind, text, image_bytes) = {
        let store = state.store.lock().unwrap();
        match store.find(&id) {
            Some(i) => {
                let bytes = store.image_file(i).and_then(|p| std::fs::read(p).ok());
                (i.kind.clone(), i.text.clone(), bytes)
            }
            None => return false,
        }
    };

    let ok = match kind.as_str() {
        "text" => {
            let t = text.unwrap_or_default();
            *state.self_set.lock().unwrap() = Some(clipboard::sig_text(&t));
            clipboard::set_text(backend, &t)
        }
        "image" => match image_bytes {
            Some(bytes) => {
                *state.self_set.lock().unwrap() = Some(clipboard::sig_image(&bytes));
                clipboard::set_image(backend, &bytes)
            }
            None => false,
        },
        // File clips store the raw `text/uri-list` payload in `text`; re-copy
        // must write it back with the `text/uri-list` mime type so file
        // managers accept the paste.
        "file" => {
            let payload = text.unwrap_or_default();
            *state.self_set.lock().unwrap() = Some(clipboard::sig_file(&payload));
            clipboard::set_files(backend, &payload)
        }
        _ => false,
    };

    // On write failure keep the overlay open — the frontend shows a notice
    // that must actually be seen.
    if ok {
        // Hide the overlay before pasting so focus returns to the target app.
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
        }
        if paste {
            // Small delay so the compositor restores focus to the previous window.
            thread::sleep(Duration::from_millis(120));
            clipboard::auto_paste(backend);
        }
    } else {
        // Nothing was written — drop the marker or it would suppress a real
        // copy of the same content. (The pre-write set still guards the poll
        // race between set_* returning and the watcher's next read.)
        *state.self_set.lock().unwrap() = None;
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
    // Expand `~` and make relative paths deterministic (HOME-based), then
    // canonicalize so `..`/symlink spellings of the same directory can't
    // reach `fs::copy(src == dst)` (which would truncate history.json).
    let new_path = store::expand_dir(&new_dir)?;
    fs::create_dir_all(&new_path)
        .map_err(|e| format!("Cannot create directory: {}", e))?;
    let new_canon = new_path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve directory: {}", e))?;

    let mut store = state.store.lock().unwrap();
    let current_dir = store.dir().to_path_buf();
    let cur_canon = current_dir
        .canonicalize()
        .map_err(|e| format!("Cannot resolve current data directory: {}", e))?;

    if cur_canon == new_canon {
        return Ok(store.settings.clone());
    }
    // Moving into a subdirectory of the current data dir would let the
    // post-migration purge delete the just-copied files.
    if new_canon.starts_with(&cur_canon) {
        return Err("Cannot move the data directory into itself".into());
    }

    let new_store = store.migrate_to(&new_canon)?;
    // Migration succeeded and the new store owns the data — drop the copy at
    // the old location. (Canonical paths are known to differ here.)
    store.purge_data();
    let settings = new_store.settings.clone();
    *store = new_store;
    drop(store);

    // Update bootstrap settings in default config dir so future startups find it.
    let default_dir = store::config_dir_for("cleepboard");
    let _ = fs::create_dir_all(&default_dir);
    let bootstrap = Settings {
        data_dir: new_canon.to_string_lossy().to_string(),
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
fn hotkey_status() -> hotkey::HotkeyStatus {
    hotkey::status()
}

/// Probe the external helper binaries cleepboard shells out to so the UI can
/// surface missing tools instead of silently degrading.
#[tauri::command]
fn check_dependencies(state: State<AppState>) -> deps::DependencyReport {
    deps::report(state.backend)
}

fn show_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        // The window is a fullscreen, transparent, undecorated, always-on-top
        // overlay: Wayland clients cannot position their own windows, so the
        // panel's placement lives entirely in CSS (`.panel` in styles.css).
        // That makes the layout identical on X11 and Wayland and gives a real
        // backdrop for click-to-dismiss. A fullscreen toplevel maps to one
        // monitor only — inherent to this approach, not fixable client-side.
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit("overlay-shown", ());
    }
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

/// Background thread: poll the clipboard signature and fetch the payload only
/// when it changes. Images are hashed over a bounded prefix so a multi-MB PNG
/// isn't re-downloaded every poll.
fn spawn_watcher(app: AppHandle) {
    thread::spawn(move || {
        let state = app.state::<AppState>();
        let backend = state.backend;
        // Seed with whatever is already on the clipboard — a launch-time poll
        // must not bump it to the top of history as if it were a new copy.
        let mut last_sig = clipboard::signature(backend);
        // Focused window seen on the previous poll: the fallback source-app
        // attribution for when our own overlay (or nothing) holds focus at
        // detection time.
        let mut prev_focus = clipboard::active_window_name(backend);

        loop {
            let poll_ms = {
                let store = state.store.lock().unwrap();
                store.settings.poll_ms.max(150)
            };
            thread::sleep(Duration::from_millis(poll_ms));

            let focused = clipboard::active_window_name(backend);
            let sig = match clipboard::signature(backend) {
                Some(s) => s,
                None => {
                    prev_focus = focused;
                    continue;
                }
            };

            // Skip if unchanged since last poll. (Checked before touching the
            // self-set marker: unchanged content must not consume it.)
            if last_sig.as_deref() == Some(sig.as_str()) {
                prev_focus = focused;
                continue;
            }
            // Skip a value we set ourselves — once. take() clears it whether
            // or not it matched: the marker exists to suppress the *echo* of
            // our write, not to veto identical content forever.
            if state.self_set.lock().unwrap().take().as_deref() == Some(sig.as_str()) {
                last_sig = Some(sig);
                prev_focus = focused;
                continue;
            }
            last_sig = Some(sig);

            // Source-app attribution is unavoidably poll-granular: the focused
            // window at detection time is our best reading; when it's the
            // overlay itself (or unreadable), fall back to the previous poll's
            // window, which usually still was the copying app. X11-only;
            // None on Wayland.
            let source_app = match &focused {
                Some(f) if f.eq_ignore_ascii_case("cleepboard") => prev_focus.clone(),
                Some(_) => focused.clone(),
                None => prev_focus.clone(),
            };

            let content = clipboard::read(backend);
            let changed = {
                let mut store = state.store.lock().unwrap();
                match content {
                    clipboard::ClipContent::Text(t) => store.add_text(t, source_app.clone()),
                    clipboard::ClipContent::Image(b) => store.add_image(&b, source_app.clone()),
                    clipboard::ClipContent::Files(uris) => store.add_file(uris, source_app.clone()),
                    clipboard::ClipContent::Empty => false,
                }
            };
            prev_focus = focused;
            if changed {
                // Emit on any history change — including the dedupe-hit path
                // (existing clip bumped to top) — or an open overlay shows a
                // stale order until reopened.
                let items = state.store.lock().unwrap().summaries();
                let _ = app.emit("history-updated", items);
            }
        }
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "Open Cleepboard", true, None::<&str>)?;
    let clear_i = MenuItem::with_id(app, "clear", "Clear history", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open_i, &clear_i, &sep, &quit_i])?;

    // Note: tray-icon 0.23's GTK/libappindicator backend wires only the menu,
    // icon, and label — it dispatches no click events and ignores
    // show_menu_on_left_click. On Linux the tray's only interaction is the
    // menu itself (left-click on appindicator shells, right-click on Plasma
    // SNI), so "Open Cleepboard" stays its first item.
    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Cleepboard")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_overlay(app),
            "clear" => {
                let state = app.state::<AppState>();
                let items = {
                    let mut store = state.store.lock().unwrap();
                    store.clear(true);
                    store.summaries()
                };
                let _ = app.emit("history-updated", items);
            }
            "quit" => app.exit(0),
            _ => {}
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
        // Same expansion rules as set_data_dir — a stored "~/…" or relative
        // path must resolve identically at startup.
        let custom = store::expand_dir(&bootstrap_settings.data_dir)
            .unwrap_or_else(|_| PathBuf::from(&bootstrap_settings.data_dir));
        if custom.join("settings.json").exists() {
            custom
        } else {
            default_dir.clone()
        }
    };

    let store = Store::load(store_dir);

    let app = tauri::Builder::default()
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
            search_history,
            get_settings,
            set_settings,
            toggle_pin,
            delete_item,
            clear_history,
            get_item_text,
            get_image_thumb,
            get_image_data_url,
            copy_item,
            hide_window,
            register_hotkey,
            unregister_hotkey,
            hotkey_status,
            check_dependencies,
            get_data_dir,
            set_data_dir,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;
            spawn_watcher(handle.clone());

            // --show and a cold-start --toggle (e.g. first hotkey press after
            // reboot, before any instance ran) both show the window; anything
            // else starts hidden in the tray.
            let args: Vec<String> = std::env::args().collect();
            let start_hidden = !args.iter().any(|a| a == "--show" || a == "--toggle");
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
                    // Esc (handled in the frontend -> hide_window), a click on
                    // the transparent backdrop outside the panel, by selecting
                    // an item, or by toggling from the tray.
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building cleepboard");

    app.run(|_app_handle, event| {
        if let RunEvent::Exit = event {
            // Reap the foreground wl-copy serving the clipboard (fallback
            // write path) so it doesn't outlive us as an orphan.
            clipboard::shutdown();
        }
    });
}

// Minimal base64 encoder (avoids pulling an extra crate).
fn base64_encode(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
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
