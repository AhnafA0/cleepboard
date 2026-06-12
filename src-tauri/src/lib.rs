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
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.center();
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

            let added = {
                let mut store = state.store.lock().unwrap();
                match content {
                    clipboard::ClipContent::Text(t) => store.add_text(t),
                    clipboard::ClipContent::Image(b) => store.add_image(&b),
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
    let backend = clipboard::detect_backend();
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
                    // Auto-hide when the overlay loses focus (click-outside / Esc-to-blur).
                    if let WindowEvent::Focused(false) = event {
                        if let Some(w) = h.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
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
