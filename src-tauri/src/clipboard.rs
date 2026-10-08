use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use crate::store;

/// The currently-live foreground `wl-copy` child that owns the clipboard
/// selection (only used on the `wl-copy` fallback path — see
/// `prefer_xclip_for_set`). We keep it attached to cleepboard's process tree
/// (via `--foreground`) instead of letting `wl-copy` daemonize into an orphan
/// that GNOME reparents to systemd and surfaces as a separate "wl-clipboard"
/// app (with its own notifications and running-app entry). Each new `set_*`
/// call kills the previous owner and stores the new one.
static WL_COPY_CHILD: Mutex<Option<Child>> = Mutex::new(None);

/// Bytes of a clipboard payload read for signature purposes only. Images are
/// capped here so a multi-MB PNG isn't re-downloaded on every poll; the full
/// payload is fetched only when the signature actually changes.
const SIG_IMAGE_CAP: usize = 64 * 1024;

/// Build a `Command` for a clipboard helper with desktop startup/activation
/// tokens stripped. Without this, every short-lived `wl-paste`/`wl-copy` we
/// spawn inherits the parent's `XDG_ACTIVATION_TOKEN`/`DESKTOP_STARTUP_ID` and
/// GNOME treats it as an app that launched but never mapped a window, spamming
/// "<app> is ready" notifications and stealing focus from our overlay.
fn helper(cmd: &str) -> Command {
    let mut c = Command::new(cmd);
    c.env_remove("XDG_ACTIVATION_TOKEN");
    c.env_remove("DESKTOP_STARTUP_ID");
    c
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
}

pub fn detect_backend() -> Backend {
    match std::env::var("XDG_SESSION_TYPE").as_deref() {
        Ok("x11") => Backend::X11,
        Ok("wayland") => Backend::Wayland,
        _ => {
            // Fall back to whichever wayland display is present.
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                Backend::Wayland
            } else {
                Backend::X11
            }
        }
    }
}

pub enum ClipContent {
    Text(String),
    Image(Vec<u8>),
    /// `file://` URIs copied from a file manager (Nautilus etc.). One clip
    /// holds all URIs from a single copy action.
    Files(Vec<String>),
    Empty,
}

// ---- Change-detection signatures ----
// The watcher compares signatures each poll; `self_set` in lib.rs stores the
// same strings to suppress our own writes. Format must stay consistent between
// the two producers — these functions are the single source of truth.

pub fn sig_text(t: &str) -> String {
    format!("t:{}", t)
}

/// Image signature over a bounded prefix — content-based (a different image
/// of the same byte length is no longer "unchanged") without hashing the
/// whole payload per poll.
pub fn sig_image(bytes: &[u8]) -> String {
    format!("i:{}", store::image_sig(bytes))
}

pub fn sig_file(payload: &str) -> String {
    format!("f:{}", payload)
}

/// Signature of the current clipboard, for the watcher's per-poll change
/// check. Mirrors `read`'s mime priority but reads images only up to
/// SIG_IMAGE_CAP bytes — the full payload is fetched via `read` once the
/// signature differs. `None` = empty or unreadable clipboard.
pub fn signature(backend: Backend) -> Option<String> {
    match backend {
        Backend::Wayland => signature_wayland(),
        Backend::X11 => signature_x11(),
    }
}

fn signature_wayland() -> Option<String> {
    let types = run_capture("wl-paste", &["--list-types"]).unwrap_or_default();
    if types.lines().any(|t| t.trim() == "image/png") {
        if let Some(bytes) =
            run_capture_prefix("wl-paste", &["--no-newline", "-t", "image/png"], SIG_IMAGE_CAP)
        {
            if !bytes.is_empty() {
                return Some(sig_image(&bytes));
            }
        }
    }
    if let Some(uris) = read_uri_list(&types, |mime| {
        run_capture("wl-paste", &["--no-newline", "-t", mime])
    }) {
        return Some(sig_file(&uris.join("\n")));
    }
    match run_capture("wl-paste", &["--no-newline", "-t", "text/plain"]) {
        Some(t) if !t.is_empty() => Some(sig_text(&t)),
        _ => None,
    }
}

fn signature_x11() -> Option<String> {
    let targets = run_capture("xclip", &["-selection", "clipboard", "-o", "-t", "TARGETS"])
        .unwrap_or_default();
    if targets.lines().any(|t| t.trim() == "image/png") {
        if let Some(bytes) = run_capture_prefix(
            "xclip",
            &["-selection", "clipboard", "-o", "-t", "image/png"],
            SIG_IMAGE_CAP,
        ) {
            if !bytes.is_empty() {
                return Some(sig_image(&bytes));
            }
        }
    }
    if let Some(uris) = read_uri_list(&targets, |mime| {
        run_capture("xclip", &["-selection", "clipboard", "-o", "-t", mime])
    }) {
        return Some(sig_file(&uris.join("\n")));
    }
    match run_capture("xclip", &["-selection", "clipboard", "-o"]) {
        Some(t) if !t.is_empty() => Some(sig_text(&t)),
        _ => None,
    }
}

/// Read the current clipboard. Priority: image/png > text/uri-list (files) >
/// text/plain. When a file manager copies an image *file* it usually offers
/// both `image/png` and `text/uri-list`; we prefer the image representation
/// there because it gives a richer thumbnail preview (see Plan 03 heuristic).
pub fn read(backend: Backend) -> ClipContent {
    match backend {
        Backend::Wayland => read_wayland(),
        Backend::X11 => read_x11(),
    }
}

fn read_wayland() -> ClipContent {
    let types = run_capture("wl-paste", &["--list-types"]).unwrap_or_default();
    let has_image = types.lines().any(|t| t.trim() == "image/png");
    if has_image {
        if let Some(bytes) = run_capture_bytes("wl-paste", &["--no-newline", "-t", "image/png"]) {
            if !bytes.is_empty() {
                return ClipContent::Image(bytes);
            }
        }
    }
    if let Some(uris) = read_uri_list(&types, |mime| {
        run_capture("wl-paste", &["--no-newline", "-t", mime])
    }) {
        return ClipContent::Files(uris);
    }
    match run_capture("wl-paste", &["--no-newline", "-t", "text/plain"]) {
        Some(t) if !t.is_empty() => ClipContent::Text(t),
        _ => ClipContent::Empty,
    }
}

fn read_x11() -> ClipContent {
    let targets = run_capture("xclip", &["-selection", "clipboard", "-o", "-t", "TARGETS"])
        .unwrap_or_default();
    let has_image = targets.lines().any(|t| t.trim() == "image/png");
    if has_image {
        if let Some(bytes) =
            run_capture_bytes("xclip", &["-selection", "clipboard", "-o", "-t", "image/png"])
        {
            if !bytes.is_empty() {
                return ClipContent::Image(bytes);
            }
        }
    }
    if let Some(uris) = read_uri_list(&targets, |mime| {
        run_capture("xclip", &["-selection", "clipboard", "-o", "-t", mime])
    }) {
        return ClipContent::Files(uris);
    }
    match run_capture("xclip", &["-selection", "clipboard", "-o"]) {
        Some(t) if !t.is_empty() => ClipContent::Text(t),
        _ => ClipContent::Empty,
    }
}

/// Collect non-empty, trimmed lines as URIs, skipping `#…` comment lines
/// (RFC 2483 — `text/uri-list` allows them; storing one as a URI would put a
/// phantom file in the clip).
fn collect_uris<'a, I: Iterator<Item = &'a str>>(lines: I) -> Vec<String> {
    lines
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// Parse a `text/uri-list` payload into individual URIs, dropping blank and
/// `#`-comment lines.
fn parse_uri_list(payload: &str) -> Vec<String> {
    collect_uris(payload.lines())
}

/// Parse the `x-special/gnome-copied-files` payload. The first line is a
/// `copy`/`cut` directive; the remaining lines are `file://` URIs. If the
/// header is missing or unrecognized, treat the whole payload as a URI list.
fn parse_gnome_copied_files(payload: &str) -> Vec<String> {
    let mut lines = payload.lines();
    match lines.next().map(str::trim) {
        Some("copy") | Some("cut") => collect_uris(lines),
        _ => parse_uri_list(payload),
    }
}

/// Extract a file-URI list from the clipboard's advertised types. Prefers the
/// standard `text/uri-list` mime; if the source offers only GNOME's
/// `x-special/gnome-copied-files`, reads that instead and strips its
/// `copy`/`cut` directive header. `read_mime` fetches the payload for a given
/// mime type. Returns `None` when no file URIs are available.
fn read_uri_list(types: &str, mut read_mime: impl FnMut(&str) -> Option<String>) -> Option<Vec<String>> {
    let has = |mime: &str| types.lines().any(|t| t.trim() == mime);
    if !has("text/uri-list") && !has("x-special/gnome-copied-files") {
        return None;
    }
    if has("text/uri-list") {
        if let Some(payload) = read_mime("text/uri-list") {
            let uris = parse_uri_list(&payload);
            if !uris.is_empty() {
                return Some(uris);
            }
        }
    }
    if has("x-special/gnome-copied-files") {
        if let Some(payload) = read_mime("x-special/gnome-copied-files") {
            let uris = parse_gnome_copied_files(&payload);
            if !uris.is_empty() {
                return Some(uris);
            }
        }
    }
    None
}

/// On GNOME/Mutter (no `ext-data-control`/`wlr-data-control`), `wl-copy` has
/// to map a 1x1 transparent `xdg_toplevel` (app-id
/// `io.github.bugaevc.wl-clipboard`) and call `gtk_surface1.present()` to grab
/// a keyboard serial before it can `set_selection()`. That popup window is what
/// GNOME surfaces as a "“wl-clipboard” is ready" notification and as a flashing
/// entry in alt-tab / the running-apps list. Per-app `gsettings` notification
/// suppression only kills the banner (not the app entry) AND races gnome-shell's
/// app cache, so it can't reliably prevent either.
///
/// We avoid the popup entirely by setting the clipboard through XWayland with
/// `xclip` instead: Mutter bridges the X CLIPBOARD selection to Wayland
/// eagerly, so `wl-paste` (and any Wayland app) sees the new value with no
/// focus demand and no mapped window. This is preferred whenever `xclip` is
/// available and an X display (XWayland) is reachable. `wl-copy` is kept as a
/// fallback for pure-Wayland systems without XWayland/xclip, where the popup is
/// unavoidable (and `suppress_wl_clipboard_notifications()` is the only lever).
fn prefer_xclip_for_set(backend: Backend) -> bool {
    match backend {
        Backend::X11 => true,
        Backend::Wayland => std::env::var("DISPLAY").is_ok() && which("xclip"),
    }
}

pub fn set_text(backend: Backend, text: &str) -> bool {
    if prefer_xclip_for_set(backend) {
        pipe_to("xclip", &["-selection", "clipboard"], text.as_bytes())
    } else {
        pipe_to("wl-copy", &[], text.as_bytes())
    }
}

pub fn set_image(backend: Backend, bytes: &[u8]) -> bool {
    if prefer_xclip_for_set(backend) {
        pipe_to(
            "xclip",
            &["-selection", "clipboard", "-t", "image/png"],
            bytes,
        )
    } else {
        pipe_to("wl-copy", &["-t", "image/png"], bytes)
    }
}

/// Write a `text/uri-list` payload back to the clipboard so file managers
/// accept the re-copy as a file paste. `payload` is the raw newline-joined
/// `file://` URI list.
pub fn set_files(backend: Backend, payload: &str) -> bool {
    if prefer_xclip_for_set(backend) {
        pipe_to(
            "xclip",
            &["-selection", "clipboard", "-t", "text/uri-list"],
            payload.as_bytes(),
        )
    } else {
        pipe_to("wl-copy", &["--type", "text/uri-list"], payload.as_bytes())
    }
}

pub fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    run_capture_bytes(cmd, args).map(|b| String::from_utf8_lossy(&b).to_string())
}

fn run_capture_bytes(cmd: &str, args: &[&str]) -> Option<Vec<u8>> {
    let out = helper(cmd).args(args).output().ok()?;
    if out.status.success() {
        Some(out.stdout)
    } else {
        None
    }
}

/// Read at most `max` bytes of a command's stdout, then stop reading. Used
/// for signatures: a multi-MB clipboard image costs 64 KiB per poll, not the
/// whole payload. The child is always reaped — closing stdout early makes
/// wl-paste/xclip die on SIGPIPE, and kill+wait covers the rest.
fn run_capture_prefix(cmd: &str, args: &[&str], max: usize) -> Option<Vec<u8>> {
    let mut child = helper(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    if let Some(mut stdout) = child.stdout.take() {
        while buf.len() < max {
            let want = (max - buf.len()).min(chunk.len());
            match stdout.read(&mut chunk[..want]) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
        drop(stdout);
    }
    let _ = child.kill();
    let _ = child.wait();
    Some(buf)
}

fn pipe_to(cmd: &str, args: &[&str], data: &[u8]) -> bool {
    // For wl-copy, run with `--foreground` so it stays a child of cleepboard
    // (in our process tree/cgroup) instead of forking into a daemon orphan
    // that GNOME surfaces as a separate "wl-clipboard" app with its own
    // notifications and running-app entry. The child is kept alive in
    // `WL_COPY_CHILD` so the clipboard value persists; the next `set_*` call
    // kills it and replaces it, and `shutdown()` reaps it on app exit.
    let is_wl_copy = cmd == "wl-copy";
    let mut full_args: Vec<&str> = Vec::with_capacity(args.len() + 1);
    if is_wl_copy {
        full_args.push("--foreground");
    }
    full_args.extend_from_slice(args);

    let child = helper(cmd)
        .args(&full_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(_) => return false,
    };
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(data).is_err() {
            // Reap: a failed-write child would otherwise sit as a zombie
            // until process exit.
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
    }
    if is_wl_copy {
        // Replace the previous foreground wl-copy owner so the new value takes
        // effect. The new child stays alive serving the selection.
        let mut slot = WL_COPY_CHILD.lock().unwrap();
        if let Some(mut prev) = slot.take() {
            let _ = prev.kill();
            let _ = prev.wait();
        }
        *slot = Some(child);
        true
    } else {
        // xclip serves the selection in the foreground; wait for it to exit
        // once the selection is replaced.
        matches!(child.wait(), Ok(s) if s.success())
    }
}

/// Kill and reap the foreground `wl-copy` serving the clipboard selection,
/// if any. Called on app exit so the child doesn't outlive cleepboard as an
/// orphan still serving the selection (and lingering as a GNOME running-app
/// entry).
pub fn shutdown() {
    if let Some(mut child) = WL_COPY_CHILD.lock().unwrap().take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Best-effort auto-paste: simulate Ctrl+V in the focused window.
pub fn auto_paste(backend: Backend) -> bool {
    match backend {
        Backend::Wayland => {
            // wtype is the common Wayland keystroke tool.
            if which("wtype") {
                return helper("wtype")
                    .args(["-M", "ctrl", "v", "-m", "ctrl"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            }
            // ydotool requires a running daemon + uinput access.
            if which("ydotool") {
                return helper("ydotool")
                    .args(["key", "29:1", "47:1", "47:0", "29:0"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            }
            false
        }
        Backend::X11 => {
            if which("xdotool") {
                return helper("xdotool")
                    .args(["key", "--clearmodifiers", "ctrl+v"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            }
            false
        }
    }
}

pub fn which(bin: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", bin))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Best-effort capture of the focused window's title — used as the "source
/// app" for clip metadata. On X11 the focused window owns the clipboard
/// selection, so `xdotool getactivewindow getwindowname` gives the copying
/// app. Returns `None` on Wayland (no reliable portal-free way to read the
/// focused app from a background process) or if xdotool is missing/fails —
/// a missing binary fails the spawn just as a `which` probe would, so this
/// stays a single spawn per call.
pub fn active_window_name(backend: Backend) -> Option<String> {
    if backend != Backend::X11 {
        return None;
    }
    let name = run_capture("xdotool", &["getactivewindow", "getwindowname"])?;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Silence GNOME's "“wl-clipboard” is ready" banner — defense-in-depth for the
/// `wl-copy` fallback path (see `prefer_xclip_for_set`). On the primary path we
/// set the clipboard via `xclip`/XWayland, which never maps a window, so this
/// is only reached on systems without XWayland/xclip.
///
/// On compositors without `ext-data-control-v1`/`wlr-data-control-v1` (stock
/// GNOME/Mutter), setting the regular clipboard selection requires a keyboard
/// serial, so `wl-copy` briefly maps an invisible 1x1 `xdg_toplevel` (app-id
/// `io.github.bugaevc.wl-clipboard`) and calls the `gtk_shell1` `present()`
/// request to ask for focus. That's unconditional in wl-clipboard's own code
/// (it doesn't depend on `XDG_ACTIVATION_TOKEN`, so stripping env vars in
/// `helper()` can't prevent it). When the compositor denies immediate focus
/// (its normal focus-stealing-prevention behavior for a brand-new window),
/// GNOME Shell's `WindowAttentionHandler` fires the "<app> is ready" toast.
///
/// gnome-shell only builds a configurable, app-specific notification policy
/// (the thing `gsettings` can actually control) for windows that resolve to
/// an app backed by a real `.desktop` file; an unmatched app-id like this one
/// falls back to the *global* notification policy, so there is normally no
/// per-app way to mute it. We work around that by installing a hidden
/// `.desktop` file under that exact app-id so GNOME resolves it to a real
/// (if `NoDisplay`) app, then pre-set that app's notification policy to
/// disabled. Best-effort and safe to call on every startup; only relevant on
/// Wayland, and a no-op if `gsettings` isn't present (not a GNOME session).
///
/// Note this only suppresses the *banner*, not the alt-tab / running-app entry
/// the popup window causes, and it races gnome-shell's app cache at startup —
/// which is exactly why the xclip path is preferred whenever available.
pub fn suppress_wl_clipboard_notifications() {
    if !which("gsettings") {
        return;
    }

    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(|h| std::path::Path::new(&h).join(".local/share")));
    let Some(data_home) = data_home else { return };

    let apps_dir = data_home.join("applications");
    if std::fs::create_dir_all(&apps_dir).is_err() {
        return;
    }
    let desktop_id = "io.github.bugaevc.wl-clipboard";
    let desktop_file = apps_dir.join(format!("{desktop_id}.desktop"));
    let contents = "[Desktop Entry]\n\
Type=Application\n\
Name=Clipboard helper\n\
Comment=Transient helper window wl-clipboard creates to set the Wayland clipboard\n\
NoDisplay=true\n\
Exec=/usr/bin/true\n\
Icon=edit-paste\n\
Terminal=false\n\
Categories=Utility;\n";
    let _ = std::fs::write(&desktop_file, contents);

    // Canonicalized exactly like gnome-shell's NotificationApplicationPolicy
    // does: lowercase, non `[a-z0-9-]` collapsed to single dashes.
    let canonical_id = desktop_id.to_lowercase().replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-");
    let schema = format!(
        "org.gnome.desktop.notifications.application:/org/gnome/desktop/notifications/application/{canonical_id}/"
    );
    let _ = Command::new("gsettings")
        .args(["set", &schema, "enable", "false"])
        .output();
    let _ = Command::new("gsettings")
        .args(["set", &schema, "application-id", &format!("{desktop_id}.desktop")])
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uri_list_drops_blanks() {
        let payload = "file:///a.txt\n\nfile:///b.txt\n";
        assert_eq!(parse_uri_list(payload), vec!["file:///a.txt", "file:///b.txt"]);
    }

    #[test]
    fn parse_uri_list_drops_rfc2483_comments() {
        // text/uri-list comment lines are metadata, not URIs.
        let payload = "#comment line\nfile:///a.txt\n# another\n";
        assert_eq!(parse_uri_list(payload), vec!["file:///a.txt"]);
    }

    #[test]
    fn parse_gnome_copied_files_strips_copy_directive() {
        let payload = "copy\nfile:///a.txt\nfile:///b.txt\n";
        assert_eq!(parse_gnome_copied_files(payload), vec!["file:///a.txt", "file:///b.txt"]);
    }

    #[test]
    fn parse_gnome_copied_files_strips_cut_directive() {
        assert_eq!(parse_gnome_copied_files("cut\nfile:///c.txt\n"), vec!["file:///c.txt"]);
    }

    #[test]
    fn parse_gnome_copied_files_without_header_falls_back_to_uri_list() {
        let payload = "file:///a.txt\nfile:///b.txt\n";
        assert_eq!(parse_gnome_copied_files(payload), vec!["file:///a.txt", "file:///b.txt"]);
    }

    #[test]
    fn read_uri_list_prefers_standard_type() {
        let types = "text/uri-list\nx-special/gnome-copied-files\n";
        let mut calls = Vec::new();
        let uris = read_uri_list(types, |mime| {
            calls.push(mime.to_string());
            (mime == "text/uri-list").then(|| "file:///standard.txt\n".to_string())
        });
        assert_eq!(uris, Some(vec!["file:///standard.txt".to_string()]));
        // Standard type had content, so the GNOME type is never consulted.
        assert_eq!(calls, vec!["text/uri-list"]);
    }

    #[test]
    fn read_uri_list_falls_back_to_gnome_type() {
        // Source advertises only the GNOME-specific type.
        let types = "x-special/gnome-copied-files\n";
        let uris = read_uri_list(types, |mime| {
            (mime == "x-special/gnome-copied-files")
                .then(|| "copy\nfile:///gnome-only.txt\n".to_string())
        });
        assert_eq!(uris, Some(vec!["file:///gnome-only.txt".to_string()]));
    }

    #[test]
    fn read_uri_list_none_when_no_file_types_advertised() {
        let types = "text/plain\nimage/png\n";
        let mut calls = 0;
        let uris = read_uri_list(types, |_| {
            calls += 1;
            None
        });
        assert_eq!(uris, None);
        // Reader must not be invoked when no file mime is advertised.
        assert_eq!(calls, 0);
    }
}
