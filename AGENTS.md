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

`xclip` must be installed (`sudo dnf install -y xclip`) — writes go through it
on both X11 and Wayland (see Conventions).

## Conventions

- Rust: minimal external deps (tauri, serde, serde_json, once_cell). Prefer
  std / small hand-rolled helpers over pulling in new crates unless clearly
  justified.
- Clipboard I/O: **reads** via `wl-paste` (Wayland) / `xclip` (X11); **writes** via
  `xclip` on both backends (on Wayland, Mutter bridges the X CLIPBOARD selection
  from XWayland, which avoids `wl-copy`'s focus-stealing popup window that GNOME
  surfaces as a "“wl-clipboard” is ready" notification + alt-tab/running-app
  entry). `wl-copy` is only a fallback when `xclip`/`DISPLAY` are unavailable
  (`prefer_xclip_for_set` in `clipboard.rs`); on that fallback path
  `suppress_wl_clipboard_notifications()` (called at startup) installs a hidden
  `.desktop` + `gsettings enable=false` as best-effort mitigation. All clipboard
  helpers are invoked via stdin/stdout pipes — never shell-interpolated, so no
  injection risk. `xclip` is now a hard runtime prerequisite (install with
  `sudo dnf install -y xclip`).
- File clips store the raw `text/uri-list` payload in `ClipItem.text`; decode
  only for the `preview` display (`percent_decode` / `uri_file_name`).
- De-dupe/self-set signatures must stay consistent between `store.add_*` and
  the watcher's `sig_*` in `lib.rs`.
