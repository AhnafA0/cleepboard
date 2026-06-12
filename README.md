# Cleepboard

A lightweight, design-first clipboard manager for Linux (Wayland & X11), built with **Tauri (Rust + web)**. UI follows the brand book in `design.pen`.

## Features

- **Clipboard history** — captures text and images automatically in the background
- **Search** — instant filtering of clip history
- **Pin** — keep important clips at the top
- **Emoji & symbol picker** — Win+V style grid
- **Global hotkey** — `Ctrl+Shift+V` opens the overlay (registered via GNOME custom shortcut)
- **Auto-paste** — selecting a clip pastes it into the focused app (best-effort on Wayland via `wtype`)
- **Tray icon** — quick open / clear / quit
- **Light / dark / system theme**, persisted history & settings

## Prerequisites (Fedora)

```bash
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
  librsvg2-devel openssl-devel curl wget file libxdo-devel \
  @development-tools wtype
```

- `wl-clipboard` (`wl-copy`/`wl-paste`) is required on Wayland — usually preinstalled on GNOME.
- `wtype` enables auto-paste on Wayland. Without it, selecting a clip just copies it (press `Ctrl+V` yourself).
- On X11, install `xclip` and `xdotool` instead.

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
npm run build    # produces deb, rpm, and AppImage under src-tauri/target/release/bundle
```

## Global hotkey setup

Open **Settings → Register shortcut** inside the app, or it can be registered manually.
It creates a GNOME custom keybinding that runs `cleepboard --toggle`, which signals the
already-running instance (via single-instance) to show/hide the overlay.

> On Wayland, apps cannot grab global shortcuts directly, so this routes through GNOME's
> own keyboard-shortcut system — the reliable approach.

## Architecture

| Layer | What it does |
|-------|--------------|
| `src-tauri/src/clipboard.rs` | Reads/writes clipboard via `wl-copy`/`wl-paste` (Wayland) or `xclip` (X11); auto-paste via `wtype`/`ydotool`/`xdotool` |
| `src-tauri/src/store.rs` | History model, de-dup, pinning, trimming, JSON persistence in `~/.config/cleepboard/` |
| `src-tauri/src/hotkey.rs` | Registers/unregisters the GNOME `gsettings` custom keybinding |
| `src-tauri/src/lib.rs` | Tauri commands, background clipboard-watcher thread, tray, single-instance toggle |
| `src/` | Vanilla HTML/CSS/JS frontend using the `design.pen` tokens |

History is stored at `~/.config/cleepboard/history.json`; images under `~/.config/cleepboard/images/`.
