use std::io::Write;
use std::process::{Command, Stdio};

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
    Empty,
}

/// Read the current clipboard. Prefers images when an image/png target is present.
pub fn read(backend: Backend) -> ClipContent {
    match backend {
        Backend::Wayland => read_wayland(),
        Backend::X11 => read_x11(),
    }
}

fn read_wayland() -> ClipContent {
    let types = run_capture("wl-paste", &["--list-types"]).unwrap_or_default();
    if types.lines().any(|t| t.trim() == "image/png") {
        if let Some(bytes) = run_capture_bytes("wl-paste", &["--no-newline", "-t", "image/png"]) {
            if !bytes.is_empty() {
                return ClipContent::Image(bytes);
            }
        }
    }
    match run_capture("wl-paste", &["--no-newline", "-t", "text/plain"]) {
        Some(t) if !t.is_empty() => ClipContent::Text(t),
        _ => ClipContent::Empty,
    }
}

fn read_x11() -> ClipContent {
    let targets = run_capture("xclip", &["-selection", "clipboard", "-o", "-t", "TARGETS"])
        .unwrap_or_default();
    if targets.lines().any(|t| t.trim() == "image/png") {
        if let Some(bytes) =
            run_capture_bytes("xclip", &["-selection", "clipboard", "-o", "-t", "image/png"])
        {
            if !bytes.is_empty() {
                return ClipContent::Image(bytes);
            }
        }
    }
    match run_capture("xclip", &["-selection", "clipboard", "-o"]) {
        Some(t) if !t.is_empty() => ClipContent::Text(t),
        _ => ClipContent::Empty,
    }
}

pub fn set_text(backend: Backend, text: &str) -> bool {
    match backend {
        Backend::Wayland => pipe_to("wl-copy", &[], text.as_bytes()),
        Backend::X11 => pipe_to("xclip", &["-selection", "clipboard"], text.as_bytes()),
    }
}

pub fn set_image(backend: Backend, bytes: &[u8]) -> bool {
    match backend {
        Backend::Wayland => pipe_to("wl-copy", &["-t", "image/png"], bytes),
        Backend::X11 => pipe_to(
            "xclip",
            &["-selection", "clipboard", "-t", "image/png"],
            bytes,
        ),
    }
}

fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    run_capture_bytes(cmd, args).map(|b| String::from_utf8_lossy(&b).to_string())
}

fn run_capture_bytes(cmd: &str, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if out.status.success() {
        Some(out.stdout)
    } else {
        None
    }
}

fn pipe_to(cmd: &str, args: &[&str], data: &[u8]) -> bool {
    let child = Command::new(cmd)
        .args(args)
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
            return false;
        }
    }
    // wl-copy daemonizes; we don't wait on it to keep the value available.
    matches!(child.wait(), Ok(s) if s.success()) || cmd == "wl-copy"
}

/// Best-effort auto-paste: simulate Ctrl+V in the focused window.
pub fn auto_paste(backend: Backend) -> bool {
    match backend {
        Backend::Wayland => {
            // wtype is the common Wayland keystroke tool.
            if which("wtype") {
                return Command::new("wtype")
                    .args(["-M", "ctrl", "v", "-m", "ctrl"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            }
            // ydotool requires a running daemon + uinput access.
            if which("ydotool") {
                return Command::new("ydotool")
                    .args(["key", "29:1", "47:1", "47:0", "29:0"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            }
            false
        }
        Backend::X11 => {
            if which("xdotool") {
                return Command::new("xdotool")
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
