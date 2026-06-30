# Cleepboard

A lightweight, design-first clipboard manager for Linux (Wayland & X11), built with **Tauri (Rust + web)**. UI follows the brand book in `design.pen`.

## Features

- **Clipboard history** — captures text, images, and copied files automatically in the background
- **File clips** — copying files in Nautilus (or any file manager) is detected via `text/uri-list` and stored as a distinct `file` clip with an extension badge; re-copying pastes the files back into a file manager. Copying an image *file* still lands as an image clip (richer thumbnail) when both mime types are offered.
- **Search** — instant filtering of clip history
- **Filter chips** — `All / Text / Images / Files / Links` quick-filter row on the history overlay
- **Pin** — keep important clips at the top
- **Clip detail** — full preview modal with metadata (timestamp + source app). Source-app capture works on X11 via `xdotool getactivewindow`; on Wayland it shows "Unknown" (no reliable portal-free way to read the focused app from a background process).
- **Emoji & symbol picker** — Win+V style grid
- **Global hotkey** — `Ctrl+Shift+V` opens the overlay (registered via GNOME custom shortcut)
- **Auto-paste** — selecting a clip pastes it into the focused app (best-effort on Wayland via `wtype`)
- **Tray icon** — quick open / clear / quit
- **Light / dark / system theme**, persisted history & settings

## Prerequisites (Fedora)

```bash
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
  librsvg2-devel openssl-devel curl wget file libxdo-devel xclip \
  @development-tools wtype
```

- `wl-clipboard` (`wl-paste`) is required on Wayland — usually preinstalled on GNOME. We use `wl-paste` to **read** the clipboard.
- `xclip` is used to **write** the clipboard on both X11 *and* Wayland (via XWayland). On GNOME/Mutter this is important: Mutter doesn't expose `ext-data-control`/`wlr-data-control`, so `wl-copy` would have to map a 1x1 transparent popup window (`app-id io.github.bugaevc.wl-clipboard`) and call `gtk_surface1.present()` to grab a serial — which GNOME surfaces as a "“wl-clipboard” is ready" notification and a flashing alt-tab/running-app entry. Routing writes through `xclip`/XWayland avoids that popup entirely (Mutter bridges the X CLIPBOARD selection to Wayland with no focus demand). `wl-copy` remains as a fallback for pure-Wayland systems without XWayland, where `suppress_wl_clipboard_notifications()` (called at startup) is the only mitigation.
- `wtype` enables auto-paste on Wayland. Without it, selecting a clip just copies it (press `Ctrl+V` yourself).
- `xdotool` is used on X11 for source-app capture and auto-paste.

Rust toolchain (if not present):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
```

## Develop

```bash
npm install
npm run dev      # runs `tauri dev`
```

## Build a release binary / packages

```bash
npm run build    # produces deb and rpm packages under src-tauri/target/release/bundle
```

## Manual install (current setup on this machine)

This machine doesn't install via the `.deb`/`.rpm` bundle — it runs a plain binary dropped in
the user's own bin dir, picked up by a per-user desktop entry:

```bash
cargo build --release --manifest-path src-tauri/Cargo.toml
pkill -x cleepboard          # stop the running instance first, otherwise the copy below fails silently
cp src-tauri/target/release/cleepboard ~/.local/bin/cleepboard
cp cleepboard.desktop ~/.local/share/applications/cleepboard.desktop
```

`~/.local/share/applications/cleepboard.desktop` and `~/.local/share/icons/cleepboard.png` are
the files GNOME actually launches — **always redeploy after rebuilding**, or you'll keep running
a stale binary that's missing the latest fixes (this bit us once already: an old build still used
`wl-copy` for writes, whose focus-stealing popup GNOME surfaced as a separate
"io.github.bugaevc.wl-clipboard" background app + "is ready" notification even while Cleepboard
itself was open; current builds write via `xclip`/XWayland to avoid that popup entirely).

There's also a leftover `/opt/cleepboard` install from an earlier system-wide attempt; it's
unused and can be removed (`sudo rm -rf /opt/cleepboard`).

## Global hotkey setup

Open **Settings → Register shortcut** inside the app, or it can be registered manually.
It creates a GNOME custom keybinding that runs `cleepboard --toggle`, which signals the
already-running instance (via single-instance) to show/hide the overlay.

> On Wayland, apps cannot grab global shortcuts directly, so this routes through GNOME's
> own keyboard-shortcut system — the reliable approach.

## Architecture

| Layer | What it does |
|-------|--------------|
| `src-tauri/src/clipboard.rs` | Reads clipboard via `wl-paste` (Wayland) or `xclip` (X11); writes via `xclip` on both backends (XWayland bridge on Wayland avoids `wl-copy`'s focus-stealing popup), with `wl-copy` as a Wayland fallback; detects `text/uri-list` file copies; auto-paste via `wtype`/`ydotool`/`xdotool` |
| `src-tauri/src/store.rs` | History model (`text` / `image` / `file` kinds), de-dup, pinning, trimming, JSON persistence in `~/.config/cleepboard/` |
| `src-tauri/src/hotkey.rs` | Registers/unregisters the GNOME `gsettings` custom keybinding |
| `src-tauri/src/lib.rs` | Tauri commands, background clipboard-watcher thread, tray, single-instance toggle |
| `src/` | Vanilla HTML/CSS/JS frontend using the `design.pen` tokens |

History is stored at `~/.config/cleepboard/history.json`; images under `~/.config/cleepboard/images/`.
