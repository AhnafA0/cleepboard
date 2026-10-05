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
- **Global hotkey** — `Ctrl+Shift+V` opens the overlay; registration is automatic on GNOME, KDE Plasma, Cinnamon, and XFCE (other desktops get copy-paste instructions in Settings)
- **Auto-paste** — selecting a clip pastes it into the focused app (`wtype`/`ydotool` on Wayland, `xdotool` on X11)
- **Tray icon** — quick open / clear / quit
- **Dependency check** — the app probes its external helpers at runtime; missing tools that break a feature surface in an overlay banner, and Settings → System tools lists every tool with a per-distro install command
- **Light / dark / system theme**, persisted history & settings

## Prerequisites

### Build dependencies

Rust toolchain (if not present):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
```

Then the Tauri system deps, per distro (Node.js is needed for `@tauri-apps/cli`):

**Ubuntu / Debian**

```bash
sudo apt-get update && sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev pkg-config build-essential curl wget file \
  nodejs npm
```

**Fedora**

```bash
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
  librsvg2-devel openssl-devel curl wget file \
  @development-tools nodejs npm
```

**Arch**

```bash
sudo pacman -Syu --needed webkit2gtk-4.1 gtk3 libappindicator-gtk3 \
  librsvg openssl pkgconf base-devel curl wget file nodejs npm
```

**openSUSE**

```bash
sudo zypper in -y webkit2gtk3-devel gtk3-devel libappindicator3-1 \
  librsvg-devel libopenssl-devel curl wget file nodejs npm
sudo zypper in -t pattern devel_basis
```

### Runtime helpers

Cleepboard doesn't link the clipboard/paste helpers — it shells out to them, and
`check_dependencies` reports which are present. Install per distro:

| Tool | apt | dnf | pacman | zypper | Used for | If missing |
|------|-----|-----|--------|--------|----------|------------|
| `wl-clipboard` | `wl-clipboard` | `wl-clipboard` | `wl-clipboard` | `wl-clipboard` | **Required on Wayland** — `wl-paste` reads the clipboard; `wl-copy` is the fallback write path | no clips appear in history; re-copy fails if `xclip` is also absent |
| `xclip` | `xclip` | `xclip` | `xclip` | `xclip` | **Required on X11** (read + write); preferred write path on Wayland (via XWayland, avoids `wl-copy`'s focus popup) | on X11: history stays empty; on Wayland: writes fall back to `wl-copy` |
| `wtype` | `wtype` | `wtype` | `wtype` | `wtype` | Optional — auto-paste on Wayland | selecting a clip only copies it; press `Ctrl+V` yourself |
| `ydotool` | `ydotool` | `ydotool` | `ydotool` | `ydotool` | Optional — alternative Wayland auto-paste | same as `wtype` (also needs the `ydotoold` daemon + uinput access) |
| `xdotool` | `xdotool` | `xdotool` | `xdotool` | `xdotool` | Optional — auto-paste + source-app name on X11 | no auto-paste; clip source shows "Unknown" |
| `gsettings` | `libglib2.0-bin` | `glib2` | `glib2` | `glib2-tools` | Optional — hotkey registration on GNOME/Cinnamon | auto-register unavailable; bind the hotkey manually (Settings shows steps) |
| `gdbus` | `libglib2.0-bin` | `glib2` | `glib2` | `glib2-tools` | Optional — hotkey registration on KDE Plasma (D-Bus transport) | KDE auto-register needs `gdbus` *or* `busctl`; otherwise bind manually |
| `busctl` | `systemd` | `systemd` | `systemd` | `systemd` | Optional — fallback transport for KDE hotkey registration | same as `gdbus` — either covers it |
| `xfconf-query` | `xfconf` | `xfconf` | `xfconf` | `xfconf` | Optional — hotkey registration on XFCE | auto-register unavailable; bind the hotkey manually |

`wl-clipboard`/`xclip`/`wtype`/`xdotool` are also listed as package `Recommends`
in the built `.deb`/`.rpm`, so installing a package manager build pulls them in
by default.

### The `xclip` write-path rationale

`xclip` is used to **write** the clipboard on both X11 *and* Wayland (via
XWayland). On GNOME/Mutter this is important: Mutter doesn't expose
`ext-data-control`/`wlr-data-control`, so `wl-copy` would have to map a 1x1
transparent popup window (`app-id io.github.bugaevc.wl-clipboard`) and call
`gtk_surface1.present()` to grab a serial — which GNOME surfaces as a
"“wl-clipboard” is ready" notification and a flashing alt-tab/running-app
entry. Routing writes through `xclip`/XWayland avoids that popup entirely
(Mutter bridges the X CLIPBOARD selection to Wayland with no focus demand).
`wl-copy` remains as a fallback for pure-Wayland systems without XWayland, where
`suppress_wl_clipboard_notifications()` (called at startup) is the only
mitigation — it hides the banner, not the running-app entry.

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
cp src-tauri/icons/icon.png ~/.local/share/icons/cleepboard.png   # Icon=cleepboard lookup
```

The `.desktop` entry uses `Exec=cleepboard --show` and `Icon=cleepboard`, which rely on
`~/.local/bin` being on the session PATH (standard on GNOME/KDE desktop sessions) and on the
icon-name lookup finding `~/.local/share/icons/cleepboard.png`.

`~/.local/share/applications/cleepboard.desktop` and `~/.local/share/icons/cleepboard.png` are
the files GNOME actually launches — **always redeploy after rebuilding**, or you'll keep running
a stale binary that's missing the latest fixes (this bit us once already: an old build still used
`wl-copy` for writes, whose focus-stealing popup GNOME surfaced as a separate
"io.github.bugaevc.wl-clipboard" background app + "is ready" notification even while Cleepboard
itself was open; current builds write via `xclip`/XWayland to avoid that popup entirely).

There's also a leftover `/opt/cleepboard` install from an earlier system-wide attempt; it's
unused and can be removed (`sudo rm -rf /opt/cleepboard`).

## Global hotkey setup

Open **Settings → Register** inside the app. `Ctrl+Shift+V` is registered through the
desktop environment's own mechanism, chosen automatically:

- **GNOME** (also Unity/Budgie): `gsettings` custom keybinding
- **KDE Plasma**: `kglobalaccel` D-Bus API + a command `.desktop` entry under `~/.local/share/kglobalaccel/`
- **Cinnamon**: `gsettings` custom keybinding (Cinnamon schema)
- **XFCE**: `xfconf-query` command shortcut

On other desktops automatic registration isn't possible — Settings then shows per-DE manual
steps to bind `cleepboard --toggle` (e.g. Sway/Hyprland get ready-to-paste config lines for
`bindsym`/`bind`). Either way the binding runs `cleepboard --toggle`, which signals the
already-running instance (via single-instance) to show/hide the overlay.

> On Wayland, apps cannot grab global shortcuts directly, so this routes through the DE's
> own keyboard-shortcut system — the reliable approach.

## Wayland vs X11 notes

The backend is auto-detected (`XDG_SESSION_TYPE`, falling back to `WAYLAND_DISPLAY`):

| | Wayland | X11 |
|---|---------|-----|
| Clipboard read | `wl-paste` | `xclip` |
| Clipboard write | `xclip` via XWayland (preferred), else `wl-copy` | `xclip` |
| Auto-paste | `wtype`, else `ydotool` | `xdotool` |
| Source-app name | "Unknown" (no portal-free way) | `xdotool getactivewindow` |

### Known compositor quirks

- **GNOME/Wayland — no auto-hide on focus loss.** The overlay deliberately does not hide when
  it loses focus: on Mutter this undecorated always-on-top surface gets `Focused(true)`
  immediately followed by `Focused(false)`, which would hide it instantly and look like a crash.
  Dismiss it with `Esc`, a click on the transparent backdrop, selecting a clip, or the tray.
- **Fullscreen surface = one monitor.** The overlay is a fullscreen transparent toplevel with
  the panel positioned in CSS (Wayland clients can't position their own windows), so it maps to
  a single monitor — inherent to the approach, not fixable client-side.
- **Wayland source-app is "Unknown".** There is no reliable portal-free way for a background
  process to read the focused app.
- **GNOME `wl-copy` fallback.** On pure-Wayland sessions without XWayland/`xclip`, writes go
  through `wl-copy`, which must map a transient window to grab a serial; the startup mitigation
  hides its "is ready" banner but not its alt-tab/running-apps entry. Installing `xclip` avoids
  this entirely on GNOME (XWayland is present by default).

## Architecture

| Layer | What it does |
|-------|--------------|
| `src-tauri/src/clipboard.rs` | Reads clipboard via `wl-paste` (Wayland) or `xclip` (X11); writes via `xclip` on both backends (XWayland bridge on Wayland avoids `wl-copy`'s focus-stealing popup), with `wl-copy` as a Wayland fallback; detects `text/uri-list` file copies; auto-paste via `wtype`/`ydotool`/`xdotool` |
| `src-tauri/src/store.rs` | History model (`text` / `image` / `file` kinds), de-dup, pinning, trimming, JSON persistence in `~/.config/cleepboard/` |
| `src-tauri/src/hotkey.rs` | Registers/unregisters the global hotkey per DE — GNOME `gsettings`, KDE `kglobalaccel` D-Bus, Cinnamon `gsettings`, XFCE `xfconf-query` — plus per-DE manual instructions |
| `src-tauri/src/deps.rs` | Probes the external helper binaries (`check_dependencies` command); models which tools cover each feature so the UI can show what's missing vs. fatal |
| `src-tauri/src/lib.rs` | Tauri commands, background clipboard-watcher thread, tray, single-instance toggle |
| `src/` | Vanilla HTML/CSS/JS frontend using the `design.pen` tokens |

History is stored at `~/.config/cleepboard/history.json`; images under `~/.config/cleepboard/images/`.
