# Plan 06 — Tray recent-clips quick access

## Goal
The system tray menu is currently Open / Clear / Quit. The design's `NoteTray`
calls for "quick access to recent clips" in the tray menu.

## Design reference (`design.pen`)
- Screen: `Screen/TrayMenu` (`L3WX9`)
  - `TrayItem1` (`q0nUv`), `TrayItem2` (`SPC9q`) — recent clip refs
  - `TrayDivider` (`Tsw49`) — a `rectangle` divider
  - `TrayItem3` (`pc0Dv`) — trailing item
- Annotation `NoteTray`:
  > System Tray/Menu Bar: Win/Linux system tray, macOS menu bar. Quick access
  > to recent clips. Open Settings, History, Quit. Context menu on right-click.

## Files to touch
- `src-tauri/src/lib.rs` — `build_tray()` (src-tauri/src/lib.rs:267) — add
  dynamic recent-clip items + divider + Settings item; rebuild menu on history
  updates.

## Implementation steps

### 1. Build a dynamic tray menu
Tauri's `Menu` is built once at startup. To show recent clips, rebuild the
tray menu whenever `history-updated` fires. Add a `rebuild_tray_menu(app)`
helper:
```rust
fn rebuild_tray_menu(app: &AppHandle) {
    let state = app.state::<AppState>();
    let recent: Vec<ClipItem> = {
        let store = state.store.lock().unwrap();
        store.items.iter().take(5).cloned().collect()
    };

    let mut items: Vec<&dyn MenuItem> = Vec::new();
    // recent clips (text only — images/files aren't useful as menu labels)
    let recents: Vec<MenuItem> = recent.iter().filter(|c| c.kind == "text").take(5).map(|c| {
        let label = c.preview.chars().take(40).collect::<String>();
        MenuItem::with_id(app, &c.id, &label, true, None::<&str>).unwrap()
    }).collect();
    // ... assemble: recents, divider, Open, Settings, Clear, Quit
    let menu = Menu::with_items(app, &refs)?;
    app.tray_by_id("main-tray").unwrap().set_menu(Some(menu)).ok();
}
```
Note: Tauri menu item IDs must be unique; reuse the clip `id` so the click
handler can look up and copy that clip.

### 2. Handle recent-clip clicks
In the existing `on_menu_event` closure (lib.rs:279), before the `"open"`/
`"clear"`/`"quit"` arms, check if the event id matches a stored clip id:
```rust
"open" => show_overlay(app),
"clear" => { ... },
"quit" => app.exit(0),
id => {
    // try to copy a clip with this id, then paste if auto_paste is on
    let state = app.state::<AppState>();
    let settings = state.store.lock().unwrap().settings.clone();
    if state.store.lock().unwrap().find(id).is_some() {
        drop(state);
        let _ = copy_item(app.clone(), app.state::<AppState>().into(), id.to_string(), settings.auto_paste);
    }
}
```
(`copy_item` is a `#[tauri::command]`; refactor the core copy logic into a
plain `fn do_copy(app, state, id, paste) -> bool` that both the command and
this handler call, to avoid the awkward `State` acquisition from a tray
callback.)

### 3. Add a Settings tray item
Add `MenuItem::with_id(app, "settings", "Settings", true, None)` and an
`"settings" =>` arm that emits an event the frontend listens for to switch to
the settings view:
```rust
"settings" => { show_overlay(app); let _ = app.emit("goto-view", "settings"); }
```
Frontend: `listen("goto-view", (e) => switchView(e.payload));`

### 4. Trigger rebuild on history changes
In `spawn_watcher` after `app.emit("history-updated", items)` (lib.rs:261),
call `rebuild_tray_menu(&app)`. Also call it once at the end of `setup()`.

## Caveats
- Tauri's GTK tray menu may have a max-item count or rendering quirks for very
  long labels — truncate previews to ~40 chars.
- Rebuilding the menu on every clipboard poll could be wasteful. Only rebuild
  when `added` is true (the existing flag at lib.rs:259) — that's already the
  gate around the `emit`, so reuse it.
- Menu items with non-UTF-8 or control chars in previews will break GTK;
  sanitize labels (replace newlines/tabs with spaces).

## Verification
1. Copy 3 text snippets → tray menu shows them as the top 3 items.
2. Click a recent-clip item in the tray → it's copied (and pasted if
   auto-paste is on) into the focused app.
3. Image clips are excluded from the tray (no useful label) — verify an image
   copy doesn't add a blank/broken tray entry.
4. Copying a 6th clip pushes the oldest out of the top-5.
5. Settings tray item opens the overlay on the Settings tab.
