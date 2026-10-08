# Cleepboard — agent notes

A lightweight, design-first clipboard manager for Linux (Tauri + vanilla JS).
Backend in Rust (`src-tauri/src`), frontend in `src/main.js`.

## Build / test / verify

Run from `src-tauri/`:

- `cargo check` — fast type/compile check
- `cargo test` — run Rust unit tests (pure functions in `#[cfg(test)] mod tests` per file)
- `cargo clippy --all-targets` — lints (treat warnings as worth fixing)
- `cargo build` / `cargo build --release` — full build (release uses LTO + `panic = abort`)

Frontend has no test runner. Verify JS syntax with `node --check src/main.js`
(run from repo root).

## Manual deploy (this machine)

This machine runs a plain binary from `~/.local/bin/cleepboard`, launched by
`~/.local/share/applications/cleepboard.desktop`. After a rebuild you **must**
redeploy or GNOME keeps launching the stale binary:

```bash
cargo build --release --manifest-path src-tauri/Cargo.toml
pkill -x cleepboard
cp src-tauri/target/release/cleepboard ~/.local/bin/cleepboard
cp cleepboard.desktop ~/.local/share/applications/cleepboard.desktop
```

The app tolerates missing helper tools (see Conventions) — install at least
`xclip` (`sudo dnf install -y xclip`) for the preferred write path, plus
`wl-clipboard`/`wtype`/`xdotool` as needed (see README's runtime-tools table).

## Conventions

- Rust: minimal external deps (tauri, serde, serde_json, image). Prefer
  std / small hand-rolled helpers over pulling in new crates unless clearly
  justified. `image` is PNG-only, used by `store.rs` for clipboard-image
  thumbnails.
- Clipboard I/O: **reads** via `wl-paste` (Wayland) / `xclip` (X11); **writes** via
  `xclip` on both backends (on Wayland, Mutter bridges the X CLIPBOARD selection
  from XWayland, which avoids `wl-copy`'s focus-stealing popup window that GNOME
  surfaces as a "“wl-clipboard” is ready" notification + alt-tab/running-app
  entry). `wl-copy` is only a fallback when `xclip`/`DISPLAY` are unavailable
  (`prefer_xclip_for_set` in `clipboard.rs`); on that fallback path
  `suppress_wl_clipboard_notifications()` (called at startup) installs a hidden
  `.desktop` + `gsettings enable=false` as best-effort mitigation. All clipboard
  helpers are invoked via stdin/stdout pipes — never shell-interpolated, so no
  injection risk. `xclip` is no longer a hard prerequisite: `deps.rs` models
  every probed tool's `ToolStatus`/`covered_by`, and the `check_dependencies`
  command feeds a System-tools list + banner in the UI so a missing tool
  degrades visibly instead of silently. Keep that report honest when adding a
  new helper call.
- Window: a fullscreen, transparent, undecorated, always-on-top overlay; the
  visible `.panel` is positioned entirely in CSS (no programmatic positioning —
  Wayland forbids it anyway), so layout is identical on X11 and Wayland and the
  transparent region doubles as a click-to-dismiss backdrop. Do NOT auto-hide
  on focus loss: GNOME/Mutter flap `Focused(true)`→`Focused(false)` on show,
  which would hide the overlay instantly (Esc/backdrop click/tray toggle
  dismiss instead).
- Global hotkey: `hotkey.rs` registers `cleepboard --toggle` per DE — GNOME
  (incl. Unity/Budgie) via `gsettings`, KDE via the `kglobalaccel` D-Bus API +
  a command desktop file, Cinnamon via its `gsettings` schema, XFCE via
  `xfconf-query`; anything else falls back to per-DE manual instructions
  surfaced through `status()`. The freedesktop GlobalShortcuts portal was
  evaluated and rejected — see the module doc comment for why.
- File clips store the raw `text/uri-list` payload in `ClipItem.text`; decode
  only for the `preview` display (`percent_decode` / `uri_file_name`).
- De-dupe/self-set signatures must stay consistent between `store.add_*` and
  the watcher's `sig_*`/`signature` helpers in `clipboard.rs`.
