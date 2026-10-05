//! Global hotkey registration per desktop environment.
//!
//! GNOME uses gsettings custom keybindings, Cinnamon its own gsettings
//! schema, XFCE xfconf command shortcuts, and KDE Plasma the kglobalaccel
//! D-Bus API (a desktop file under ~/.local/share/kglobalaccel/ plus
//! setForeignShortcut* calls so the binding applies live). Everything else
//! gets manual per-DE instructions surfaced through `status()`.
//!
//! The freedesktop `org.freedesktop.portal.GlobalShortcuts` portal was
//! evaluated as the portable path and deliberately not used: BindShortcuts
//! only opens a dialog for the *user* to pick the trigger — an app cannot
//! register a fixed binding programmatically — and activation requires a
//! persistent in-process D-Bus signal listener for `Activated`. Backend
//! coverage is also inconsistent (xdg-desktop-portal-gnome does not
//! implement GlobalShortcuts at all; only KDE and Hyprland do), so the
//! per-DE mechanisms here stay the better fit.

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::clipboard::{run_capture, which};

// ---- GNOME ----
const GNOME_KB_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/cleepboard/";
const GNOME_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const GNOME_CUSTOM_SCHEMA: &str =
    "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";

// ---- Cinnamon ----
const CIN_SCHEMA: &str = "org.cinnamon.desktop.keybindings";
const CIN_CUSTOM_SCHEMA: &str = "org.cinnamon.desktop.keybindings.custom-keybinding";
const CIN_ID: &str = "cleepboard";
const CIN_PATH: &str = "/org/cinnamon/desktop/keybindings/custom-keybindings/cleepboard/";

// ---- KDE ----
// Command shortcuts live in a desktop file the daemon resolves via
// XDG data dirs (no ksycoca rebuild needed): ~/.local/share/kglobalaccel/.
const KDE_COMPONENT: &str = "net.local.cleepboard-toggle.desktop";
const KDE_DEST: &str = "org.kde.kglobalaccel";
const KDE_OBJ: &str = "/kglobalaccel";
const KDE_IFACE: &str = "org.kde.KGlobalAccel";

// ---- XFCE ----
const XFCE_CHANNEL: &str = "xfce4-keyboard-shortcuts";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mechanism {
    Gnome,
    Kde,
    Cinnamon,
    Xfce,
    Manual,
}

impl Mechanism {
    fn name(self) -> &'static str {
        match self {
            Mechanism::Gnome => "gnome",
            Mechanism::Kde => "kde",
            Mechanism::Cinnamon => "cinnamon",
            Mechanism::Xfce => "xfce",
            Mechanism::Manual => "manual",
        }
    }
}

/// XDG_CURRENT_DESKTOP may be a colon-separated list ("Unity:Unity7:ubuntu");
/// matched on presence of the name, in GNOME → KDE → Cinnamon → XFCE order.
fn detect_mechanism(desktop: &str) -> Mechanism {
    let d = desktop.to_lowercase();
    let has = |needle: &str| d.split(':').any(|t| t.trim().contains(needle));
    if has("gnome") || has("unity") || has("budgie") {
        Mechanism::Gnome
    } else if has("kde") || has("plasma") {
        Mechanism::Kde
    } else if has("cinnamon") {
        Mechanism::Cinnamon
    } else if has("xfce") {
        Mechanism::Xfce
    } else {
        Mechanism::Manual
    }
}

fn current_desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()
}

pub(crate) fn current_mechanism() -> Mechanism {
    detect_mechanism(&current_desktop())
}

/// Canonical binding: GTK-style accelerators like `<Control><Shift>v`
/// (what the frontend sends). Translated per mechanism.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Binding {
    ctrl: bool,
    shift: bool,
    alt: bool,
    supr: bool,
    key: String,
}

impl Binding {
    fn parse(s: &str) -> Option<Binding> {
        let mut b = Binding {
            ctrl: false,
            shift: false,
            alt: false,
            supr: false,
            key: String::new(),
        };
        let mut rest = s.trim();
        while rest.starts_with('<') {
            let end = rest.find('>')?;
            match rest[1..end].to_lowercase().as_str() {
                "control" | "ctrl" | "primary" => b.ctrl = true,
                "shift" => b.shift = true,
                "alt" => b.alt = true,
                "super" | "meta" | "win" | "mod4" => b.supr = true,
                _ => return None,
            }
            rest = rest[end + 1..].trim_start();
        }
        if rest.is_empty() {
            return None;
        }
        b.key = rest.to_string();
        Some(b)
    }

    /// GTK/gsettings accel: `<Control><Shift>v` (GNOME, Cinnamon).
    fn gtk(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("<Control>");
        }
        if self.alt {
            s.push_str("<Alt>");
        }
        if self.shift {
            s.push_str("<Shift>");
        }
        if self.supr {
            s.push_str("<Super>");
        }
        s.push_str(&self.key);
        s
    }

    /// KDE accel string: `Ctrl+Shift+V`.
    fn kde(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.supr {
            parts.push("Meta".to_string());
        }
        parts.push(kde_key_name(&self.key));
        parts.join("+")
    }

    /// XFCE accel string: `<Primary><Shift>v` (Primary = Ctrl).
    fn xfce(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("<Primary>");
        }
        if self.alt {
            s.push_str("<Alt>");
        }
        if self.shift {
            s.push_str("<Shift>");
        }
        if self.supr {
            s.push_str("<Super>");
        }
        s.push_str(&self.key);
        s
    }

    /// Qt combined key integer (modifiers | Qt::Key) as used by kglobalaccel's
    /// `a(ai)` D-Bus signature — e.g. Ctrl+Shift+V = 0x06000056.
    fn qt_combined(&self) -> Option<u32> {
        let mut code = qt_key(&self.key)?;
        if self.ctrl {
            code |= 0x0400_0000;
        }
        if self.shift {
            code |= 0x0200_0000;
        }
        if self.alt {
            code |= 0x0800_0000;
        }
        if self.supr {
            code |= 0x1000_0000;
        }
        Some(code)
    }
}

/// Qt display name for a key ("v" -> "V", "space" -> "Space", ...).
fn kde_key_name(key: &str) -> String {
    let lower = key.to_lowercase();
    if lower.chars().count() == 1 {
        return lower.to_uppercase();
    }
    if lower
        .strip_prefix('f')
        .and_then(|d| d.parse::<u32>().ok())
        .map(|n| (1..=24).contains(&n))
        .unwrap_or(false)
    {
        return lower.to_uppercase();
    }
    match lower.as_str() {
        "escape" | "esc" => "Escape".into(),
        "return" | "enter" => "Return".into(),
        "space" => "Space".into(),
        "tab" => "Tab".into(),
        "backspace" => "Backspace".into(),
        "delete" | "del" => "Delete".into(),
        "insert" | "ins" => "Insert".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "page_up" | "pageup" | "prior" => "PageUp".into(),
        "page_down" | "pagedown" | "next" => "PageDown".into(),
        "left" => "Left".into(),
        "up" => "Up".into(),
        "right" => "Right".into(),
        "down" => "Down".into(),
        "print" | "printscreen" => "Print".into(),
        _ => {
            let mut c = key.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => key.to_string(),
            }
        }
    }
}

/// Qt::Key code for a key name. ASCII characters map to their code (letters
/// uppercased); common named keys map to the Qt::Key_* enum values.
fn qt_key(name: &str) -> Option<u32> {
    let lower = name.to_lowercase();
    if lower.chars().count() == 1 {
        let c = lower.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return Some(c.to_ascii_uppercase() as u32);
        }
        if c.is_ascii() {
            return Some(c as u32);
        }
        return None;
    }
    if let Some(digits) = lower.strip_prefix('f') {
        if let Ok(n) = digits.parse::<u32>() {
            if (1..=24).contains(&n) {
                return Some(0x0100_0030 + n - 1);
            }
        }
    }
    let code = match lower.as_str() {
        "escape" | "esc" => 0x0100_0000,
        "tab" => 0x0100_0001,
        "backtab" => 0x0100_0002,
        "backspace" => 0x0100_0003,
        "return" | "enter" => 0x0100_0004,
        "insert" | "ins" => 0x0100_0006,
        "delete" | "del" => 0x0100_0007,
        "pause" | "break" => 0x0100_0008,
        "print" | "printscreen" | "sysrq" => 0x0100_0009,
        "home" => 0x0100_0010,
        "end" => 0x0100_0011,
        "left" => 0x0100_0012,
        "up" => 0x0100_0013,
        "right" => 0x0100_0014,
        "down" => 0x0100_0015,
        "page_up" | "pageup" | "prior" => 0x0100_0016,
        "page_down" | "pagedown" | "next" => 0x0100_0017,
        "capslock" => 0x0100_0024,
        "numlock" => 0x0100_0025,
        "scrolllock" => 0x0100_0026,
        "space" => 0x20,
        "minus" => '-' as u32,
        "equal" => '=' as u32,
        "comma" => ',' as u32,
        "period" | "fullstop" => '.' as u32,
        "slash" => '/' as u32,
        "backslash" => '\\' as u32,
        "semicolon" => ';' as u32,
        "apostrophe" | "quote" => '\'' as u32,
        "bracketleft" => '[' as u32,
        "bracketright" => ']' as u32,
        "grave" | "backquote" => '`' as u32,
        _ => return None,
    };
    Some(code)
}

fn gsettings(args: &[&str]) -> Option<String> {
    let out = Command::new("gsettings").args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn has_gsettings() -> bool {
    which("gsettings")
}

fn current_exe_string() -> Result<String, String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

fn exe_path() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "cleepboard".to_string())
}

/// Quote for command strings later re-parsed g_shell_parse_argv-style:
/// gsettings `command` values (GNOME/Cinnamon) and xfconf shortcut commands
/// (XFCE). A bare-safe value passes through untouched; anything else is
/// wrapped in single quotes (a `'` inside becomes the standard '\'' form)
/// so a spaced install path stays one argument.
fn gshell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// `{exe} --toggle` for gsettings/xfconf command values.
fn exe_toggle_shell_cmd() -> Result<String, String> {
    Ok(format!("{} --toggle", gshell_quote(&current_exe_string()?)))
}

/// Register the global hotkey that runs `<exe> --toggle`. The binding uses
/// the canonical GTK accel format (`<Control><Shift>v`) and is translated
/// per mechanism. Returns a human-readable status string.
pub fn register(binding: &str) -> Result<String, String> {
    let b = Binding::parse(binding)
        .ok_or_else(|| format!("unrecognized binding {:?} (expected e.g. <Control><Shift>v)", binding))?;
    match current_mechanism() {
        Mechanism::Gnome => gnome_register(&b),
        Mechanism::Kde => kde_register(&b),
        Mechanism::Cinnamon => cinnamon_register(&b),
        Mechanism::Xfce => xfce_register(&b),
        Mechanism::Manual => Err(format!(
            "no automatic hotkey mechanism on this desktop ({}) — see the manual steps in Settings",
            current_desktop()
        )),
    }
}

pub fn unregister() -> Result<String, String> {
    match current_mechanism() {
        Mechanism::Gnome => gnome_unregister(),
        Mechanism::Kde => kde_unregister(),
        Mechanism::Cinnamon => cinnamon_unregister(),
        Mechanism::Xfce => xfce_unregister(),
        Mechanism::Manual => Ok("nothing to unregister".into()),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HotkeyStatus {
    pub mechanism: &'static str,
    pub registered: bool,
    /// Concrete manual steps for this desktop (always populated so the UI can
    /// show them as a fallback even when an automatic mechanism exists).
    pub instructions: String,
}

pub fn status() -> HotkeyStatus {
    let mech = current_mechanism();
    let registered = match mech {
        Mechanism::Gnome => gnome_is_registered(),
        Mechanism::Kde => kde_is_registered(),
        Mechanism::Cinnamon => cinnamon_is_registered(),
        Mechanism::Xfce => xfce_is_registered(),
        Mechanism::Manual => false,
    };
    HotkeyStatus {
        mechanism: mech.name(),
        registered,
        instructions: manual_instructions(&current_desktop(), &exe_path()),
    }
}

// ==================== GNOME ====================

fn gnome_register(b: &Binding) -> Result<String, String> {
    if !has_gsettings() {
        return Err("gsettings not found (not a GNOME session)".into());
    }
    let command = exe_toggle_shell_cmd()?;
    let binding = b.gtk();

    // Read the current list of custom keybinding paths.
    let current = gsettings(&["get", GNOME_SCHEMA, "custom-keybindings"])
        .unwrap_or_else(|| "@as []".into());

    let mut paths: Vec<String> = parse_gvariant_list(&current);
    if !paths.iter().any(|p| p == GNOME_KB_PATH) {
        paths.push(GNOME_KB_PATH.to_string());
    }
    let new_list = format_gvariant_list(&paths);
    gsettings(&["set", GNOME_SCHEMA, "custom-keybindings", &new_list])
        .ok_or("failed to set custom-keybindings list")?;

    let rel = format!("{}:{}", GNOME_CUSTOM_SCHEMA, GNOME_KB_PATH);
    gsettings(&["set", &rel, "name", "Cleepboard Toggle"]).ok_or("failed to set name")?;
    gsettings(&["set", &rel, "command", &command]).ok_or("failed to set command")?;
    gsettings(&["set", &rel, "binding", &binding]).ok_or("failed to set binding")?;

    Ok(format!("Registered {} -> {}", binding, command))
}

fn gnome_unregister() -> Result<String, String> {
    if !has_gsettings() {
        return Err("gsettings not found".into());
    }
    let current = gsettings(&["get", GNOME_SCHEMA, "custom-keybindings"])
        .unwrap_or_else(|| "@as []".into());
    let paths: Vec<String> = parse_gvariant_list(&current)
        .into_iter()
        .filter(|p| p != GNOME_KB_PATH)
        .collect();
    let new_list = format_gvariant_list(&paths);
    gsettings(&["set", GNOME_SCHEMA, "custom-keybindings", &new_list])
        .ok_or("failed to update list")?;
    Ok("Unregistered".into())
}

fn gnome_is_registered() -> bool {
    if !has_gsettings() {
        return false;
    }
    let current = gsettings(&["get", GNOME_SCHEMA, "custom-keybindings"]).unwrap_or_default();
    parse_gvariant_list(&current)
        .iter()
        .any(|p| p == GNOME_KB_PATH)
}

// ==================== KDE Plasma ====================
//
// kglobalaccel binds a command shortcut via a .desktop file whose `_launch`
// action is triggered on keypress. We write the file under
// $XDG_DATA_HOME/kglobalaccel/ (resolved via QStandardPaths on both Plasma 5
// and 6 — no ksycoca rebuild), then bind through the D-Bus API so it applies
// live (editing kglobalshortcutsrc alone only takes effect at daemon start).
// qdbus can't marshal the `as`/`a(ai)` argument types, so we use gdbus
// (glib) with a busctl (systemd) fallback.

fn kde_desktop_path() -> PathBuf {
    let data = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        });
    data.join("kglobalaccel").join(KDE_COMPONENT)
}

fn kde_config_path() -> PathBuf {
    let cfg = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".config")
        });
    cfg.join("kglobalshortcutsrc")
}

/// Double-quote one argument in a desktop-file `Exec=` line, per the
/// Desktop Entry spec: a value holding a reserved char is wrapped in `"`
/// with `\`, `"`, `` ` `` and `$` backslash-escaped.
fn desktop_entry_quote(s: &str) -> String {
    if !s.is_empty()
        && !s.chars().any(|c| {
            c.is_whitespace()
                || matches!(
                    c,
                    '"' | '\'' | '\\' | '>' | '<' | '~' | '|' | '&' | ';' | '$' | '*' | '?'
                        | '#' | '(' | ')' | '`'
                )
        })
    {
        return s.to_string();
    }
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    format!("\"{}\"", escaped)
}

fn kde_write_desktop_file(exe: &str) -> Result<(), String> {
    let path = kde_desktop_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let exec = format!("{} --toggle", desktop_entry_quote(exe));
    let content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Cleepboard Toggle\n\
         Exec={}\n\
         NoDisplay=true\n\
         StartupNotify=false\n\
         X-KDE-GlobalAccel-CommandShortcut=true\n",
        exec
    );
    fs::write(&path, content).map_err(|e| e.to_string())?;
    // ApplicationLauncherJob may refuse non-executable desktop files.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

/// Call a method on org.kde.KGlobalAccel via gdbus, or busctl when gdbus is
/// unavailable. `gdbus_args` are GVariant literals; `busctl_sig`/`busctl_args`
/// are the busctl signature + flat argument list.
fn kga_call(
    method: &str,
    gdbus_args: &[String],
    busctl_sig: &str,
    busctl_args: &[String],
) -> Option<String> {
    if which("gdbus") {
        let mut argv: Vec<String> = vec![
            "call".into(),
            "--session".into(),
            "--dest".into(),
            KDE_DEST.into(),
            "--object-path".into(),
            KDE_OBJ.into(),
            "--method".into(),
            format!("{}.{}", KDE_IFACE, method),
        ];
        argv.extend(gdbus_args.iter().cloned());
        let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        return run_capture("gdbus", &refs);
    }
    if which("busctl") {
        let mut argv: Vec<String> = vec![
            "--user".into(),
            "call".into(),
            KDE_DEST.into(),
            KDE_OBJ.into(),
            KDE_IFACE.into(),
            method.into(),
            busctl_sig.into(),
        ];
        argv.extend(busctl_args.iter().cloned());
        let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        return run_capture("busctl", &refs);
    }
    None
}

fn kde_action_id() -> String {
    format!(
        "['{}', '_launch', 'Cleepboard', 'Cleepboard Toggle']",
        KDE_COMPONENT
    )
}

fn kde_register(b: &Binding) -> Result<String, String> {
    if !which("gdbus") && !which("busctl") {
        return Err(
            "KDE registration needs gdbus or busctl to reach kglobalaccel".into(),
        );
    }
    let exe = current_exe_string()?;
    kde_write_desktop_file(&exe)?;
    // Best-effort service-cache refresh so the entry also shows in
    // System Settings' custom shortcuts; harmless if absent.
    let _ = run_capture("kbuildsycoca6", &["--noincremental"]);
    let _ = run_capture("kbuildsycoca5", &["--noincremental"]);

    let key = b
        .qt_combined()
        .ok_or_else(|| format!("unsupported key for KDE shortcut: {}", b.key))?;
    // doRegister of a dummy action forces the daemon to load the desktop file
    // (its `_launch` action is registered as a side effect), then we remove
    // the dummy and bind the real action. setForeignShortcutKeys exists on
    // KF6 and KF5 ≥5.90; older Plasma 5 exposes setForeignShortcut instead.
    let dummy = format!("['{}', '', 'Cleepboard', '']", KDE_COMPONENT);
    let action = kde_action_id();
    let _ = kga_call(
        "doRegister",
        std::slice::from_ref(&dummy),
        "as",
        &["4".into(), KDE_COMPONENT.into(), "".into(), "Cleepboard".into(), "".into()],
    );
    let bound = kga_call(
        "setForeignShortcutKeys",
        &[action.clone(), format!("[([{}],)]", key)],
        "asa(ai)",
        &[
            "4".into(),
            KDE_COMPONENT.into(),
            "_launch".into(),
            "Cleepboard".into(),
            "Cleepboard Toggle".into(),
            "1".into(),
            "1".into(),
            key.to_string(),
        ],
    )
    .is_some()
        || kga_call(
            "setForeignShortcut",
            &[action, format!("[{}]", key)],
            "asai",
            &[
                "4".into(),
                KDE_COMPONENT.into(),
                "_launch".into(),
                "Cleepboard".into(),
                "Cleepboard Toggle".into(),
                "1".into(),
                key.to_string(),
            ],
        )
        .is_some();
    if !bound {
        return Err("kglobalaccel D-Bus call failed — is a Plasma session running?".into());
    }
    Ok(format!("Registered {} -> {} --toggle", b.kde(), exe))
}

fn kde_unregister() -> Result<String, String> {
    // Remove the shortcut in the running daemon (Plasma ≥5.90 `unregister`,
    // older `unRegister`); both are no-ops if it isn't bound.
    let ok = kga_call(
        "unregister",
        &[
            format!("'{}'", KDE_COMPONENT),
            "'_launch'".to_string(),
        ],
        "ss",
        &[KDE_COMPONENT.into(), "_launch".into()],
    )
    .is_some()
        || kga_call(
            "unRegister",
            &[kde_action_id()],
            "as",
            &[
                "4".into(),
                KDE_COMPONENT.into(),
                "_launch".into(),
                "Cleepboard".into(),
                "Cleepboard Toggle".into(),
            ],
        )
        .is_some();
    let _ = fs::remove_file(kde_desktop_path());
    // Strip the persisted group too, so state is gone even when the daemon
    // wasn't running to prune it itself. KF5 uses a flat [name.desktop]
    // group; KF6 nests it under [services].
    if let Ok(content) = fs::read_to_string(kde_config_path()) {
        let stripped = strip_ini_sections(
            &content,
            &[
                format!("[{}]", KDE_COMPONENT),
                format!("[services][{}]", KDE_COMPONENT),
            ],
        );
        let _ = fs::write(kde_config_path(), stripped);
    }
    if ok {
        Ok("Unregistered".into())
    } else {
        Ok("Unregistered (daemon not reachable; removed stored config)".into())
    }
}

/// The bound key string for our `_launch` action, if kglobalshortcutsrc has
/// one. First comma-separated field is the active shortcut; "none"/empty
/// means explicitly unbound.
fn kde_launch_value(content: &str) -> Option<String> {
    let headers = [
        format!("[{}]", KDE_COMPONENT),
        format!("[services][{}]", KDE_COMPONENT),
    ];
    let mut in_section = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = headers.iter().any(|h| t == h.as_str());
        } else if in_section && t.starts_with("_launch=") {
            let v = t["_launch=".len()..]
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            return if v.is_empty() || v.eq_ignore_ascii_case("none") {
                None
            } else {
                Some(v)
            };
        }
    }
    None
}

fn kde_is_registered() -> bool {
    fs::read_to_string(kde_config_path())
        .ok()
        .and_then(|c| kde_launch_value(&c))
        .is_some()
}

fn strip_ini_sections(content: &str, headers: &[String]) -> String {
    let mut out = String::new();
    let mut skip = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            skip = headers.iter().any(|h| t == h.as_str());
            if skip {
                continue;
            }
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

// ==================== Cinnamon ====================

fn cinnamon_register(b: &Binding) -> Result<String, String> {
    if !has_gsettings() {
        return Err("gsettings not found (not a Cinnamon session)".into());
    }
    let command = exe_toggle_shell_cmd()?;
    let current =
        gsettings(&["get", CIN_SCHEMA, "custom-list"]).unwrap_or_else(|| "@as []".into());
    let mut ids = parse_gvariant_list(&current);
    if !ids.iter().any(|i| i == CIN_ID) {
        ids.push(CIN_ID.to_string());
    }
    gsettings(&["set", CIN_SCHEMA, "custom-list", &format_gvariant_list(&ids)])
        .ok_or("failed to set custom-list")?;

    let rel = format!("{}:{}", CIN_CUSTOM_SCHEMA, CIN_PATH);
    gsettings(&["set", &rel, "name", "Cleepboard Toggle"]).ok_or("failed to set name")?;
    gsettings(&["set", &rel, "command", &command]).ok_or("failed to set command")?;
    // `binding` is a string array (a shortcut can have several bindings).
    gsettings(&["set", &rel, "binding", &format!("['{}']", b.gtk())])
        .ok_or("failed to set binding")?;

    Ok(format!("Registered {} -> {}", b.gtk(), command))
}

fn cinnamon_unregister() -> Result<String, String> {
    if !has_gsettings() {
        return Err("gsettings not found".into());
    }
    let current =
        gsettings(&["get", CIN_SCHEMA, "custom-list"]).unwrap_or_else(|| "@as []".into());
    let ids: Vec<String> = parse_gvariant_list(&current)
        .into_iter()
        .filter(|i| i != CIN_ID)
        .collect();
    gsettings(&["set", CIN_SCHEMA, "custom-list", &format_gvariant_list(&ids)])
        .ok_or("failed to update custom-list")?;
    let rel = format!("{}:{}", CIN_CUSTOM_SCHEMA, CIN_PATH);
    let _ = gsettings(&["set", &rel, "binding", "[]"]);
    Ok("Unregistered".into())
}

fn cinnamon_is_registered() -> bool {
    if !has_gsettings() {
        return false;
    }
    let current = gsettings(&["get", CIN_SCHEMA, "custom-list"]).unwrap_or_default();
    parse_gvariant_list(&current).iter().any(|i| i == CIN_ID)
}

// ==================== XFCE ====================
// Command shortcuts are xfconf properties under
// /commands/custom/<accel> where <accel> is the GTK-style binding
// (`<Primary>` = Ctrl).

fn xfce_query(args: &[&str]) -> Option<String> {
    run_capture("xfconf-query", args)
}

fn xfce_register(b: &Binding) -> Result<String, String> {
    if !which("xfconf-query") {
        return Err("xfconf-query not found (not an XFCE session)".into());
    }
    let command = exe_toggle_shell_cmd()?;
    let prop = format!("/commands/custom/{}", b.xfce());
    // -n creates a new property (fails if it already exists); plain -s
    // covers the existing case. Try both.
    let ok = xfce_query(&[
        "-n", "-c", XFCE_CHANNEL, "-p", &prop, "-t", "string", "-s", &command,
    ])
    .is_some()
        || xfce_query(&["-c", XFCE_CHANNEL, "-p", &prop, "-s", &command]).is_some();
    if !ok {
        return Err("xfconf-query failed to set the shortcut property".into());
    }
    Ok(format!("Registered {} -> {}", b.xfce(), command))
}

/// `/commands/custom/*` properties whose command launches cleepboard
/// (`xfconf-query -lv` output is "prop<spaces>value" per line).
fn xfce_props(list_output: &str, command: &str) -> Vec<String> {
    list_output
        .lines()
        .filter_map(|line| {
            let idx = line.find(|c: char| c.is_whitespace())?;
            let (prop, val) = (line[..idx].trim(), line[idx..].trim());
            if prop.starts_with("/commands/custom/")
                && (val == command || (val.contains("cleepboard") && val.contains("--toggle")))
            {
                Some(prop.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn xfce_unregister() -> Result<String, String> {
    if !which("xfconf-query") {
        return Err("xfconf-query not found".into());
    }
    let command = exe_toggle_shell_cmd()?;
    let list = xfce_query(&["-c", XFCE_CHANNEL, "-lv"]).unwrap_or_default();
    let props = xfce_props(&list, &command);
    for prop in &props {
        let _ = xfce_query(&["-c", XFCE_CHANNEL, "-p", prop, "-r"]);
    }
    Ok("Unregistered".into())
}

fn xfce_is_registered() -> bool {
    if !which("xfconf-query") {
        return false;
    }
    let command = match exe_toggle_shell_cmd() {
        Ok(c) => c,
        Err(_) => return false,
    };
    xfce_query(&["-c", XFCE_CHANNEL, "-lv"])
        .map(|list| !xfce_props(&list, &command).is_empty())
        .unwrap_or(false)
}

// ==================== manual instructions ====================

fn manual_instructions(desktop: &str, exe: &str) -> String {
    let d = desktop.to_lowercase();
    if d.contains("gnome") || d.contains("unity") || d.contains("budgie") {
        format!(
            "Settings → Keyboard → Keyboard Shortcuts → Custom Shortcuts → Add\n  Command: {} --toggle\n  Shortcut: Ctrl+Shift+V",
            exe
        )
    } else if d.contains("hyprland") {
        format!(
            "Add to ~/.config/hypr/hyprland.conf:\n\n    bind = CTRL SHIFT, V, exec, {} --toggle\n\nThen run: hyprctl reload",
            exe
        )
    } else if d.contains("sway") {
        format!(
            "Add to ~/.config/sway/config:\n\n    bindsym Ctrl+Shift+v exec {} --toggle\n\nThen run: swaymsg reload",
            exe
        )
    } else if d.contains("i3") {
        format!(
            "Add to ~/.config/i3/config:\n\n    bindsym Ctrl+Shift+v exec {} --toggle\n\nThen run: i3-msg reload",
            exe
        )
    } else if d.contains("lxqt") {
        format!(
            "LXQt Settings → Shortcut Keys → Add\n  Command: {} --toggle\n  Shortcut: Ctrl+Shift+V",
            exe
        )
    } else if d.contains("kde") || d.contains("plasma") {
        format!(
            "System Settings → Shortcuts → Add Command…\n  Command: {} --toggle\n  Trigger: Ctrl+Shift+V",
            exe
        )
    } else if d.contains("cinnamon") {
        format!(
            "System Settings → Keyboard → Shortcuts → Custom Shortcuts → Add custom shortcut\n  Command: {} --toggle\n  Binding: Ctrl+Shift+V",
            exe
        )
    } else if d.contains("xfce") {
        format!(
            "Settings → Keyboard → Application Shortcuts → Add\n  Command: {} --toggle\n  Shortcut: Ctrl+Shift+V",
            exe
        )
    } else if d.contains("mate") {
        format!(
            "System → Preferences → Hardware → Keyboard Shortcuts → Add\n  Command: {} --toggle\n  Shortcut: Ctrl+Shift+V",
            exe
        )
    } else if d.contains("openbox") || d.contains("lxde") {
        format!(
            "Add a keybind running `{} --toggle` to your openbox rc.xml keyboard section, or use obkey.",
            exe
        )
    } else {
        format!(
            "This desktop has no automatic registration mechanism.\nBind `{} --toggle` to Ctrl+Shift+V in its keyboard settings.",
            exe
        )
    }
}

// ==================== shared parsing ====================

// GVariant array-of-strings looks like: ['a', 'b'] or @as [] when empty.
fn parse_gvariant_list(s: &str) -> Vec<String> {
    let s = s.trim();
    let start = match s.find('[') {
        Some(i) => i,
        None => return vec![],
    };
    let end = match s.rfind(']') {
        Some(i) => i,
        None => return vec![],
    };
    if end <= start {
        return vec![];
    }
    let inner = &s[start + 1..end];
    inner
        .split(',')
        .map(|p| p.trim().trim_matches('\'').trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn format_gvariant_list(paths: &[String]) -> String {
    if paths.is_empty() {
        return "[]".to_string();
    }
    let joined = paths
        .iter()
        .map(|p| format!("'{}'", p))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]", joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_binding_modifiers() {
        let b = Binding::parse("<Control><Shift>v").unwrap();
        assert!(b.ctrl && b.shift && !b.alt && !b.supr);
        assert_eq!(b.key, "v");
        let b = Binding::parse("<Primary><Alt>x").unwrap();
        assert!(b.ctrl && b.alt && !b.shift);
        assert_eq!(b.key, "x");
        let b = Binding::parse("<Super>space").unwrap();
        assert!(b.supr);
        assert_eq!(b.key, "space");
        let b = Binding::parse("F5").unwrap();
        assert!(!b.ctrl && b.key == "F5");
    }

    #[test]
    fn parse_binding_rejects_bad() {
        assert!(Binding::parse("").is_none());
        assert!(Binding::parse("<Control>").is_none());
        assert!(Binding::parse("<Bogus>x").is_none());
    }

    #[test]
    fn binding_formats() {
        let b = Binding::parse("<Control><Shift>v").unwrap();
        assert_eq!(b.gtk(), "<Control><Shift>v");
        assert_eq!(b.kde(), "Ctrl+Shift+V");
        assert_eq!(b.xfce(), "<Primary><Shift>v");

        let b = Binding::parse("<Primary><Alt>F9").unwrap();
        assert_eq!(b.gtk(), "<Control><Alt>F9");
        assert_eq!(b.kde(), "Ctrl+Alt+F9");
        assert_eq!(b.xfce(), "<Primary><Alt>F9");

        let b = Binding::parse("<Super>space").unwrap();
        assert_eq!(b.kde(), "Meta+Space");
    }

    #[test]
    fn qt_combined_codes() {
        // Qt::CTRL | Qt::SHIFT | Qt::Key_V
        let b = Binding::parse("<Control><Shift>v").unwrap();
        assert_eq!(b.qt_combined(), Some(0x0400_0000 | 0x0200_0000 | 0x56));
        let b = Binding::parse("<Alt>F1").unwrap();
        assert_eq!(b.qt_combined(), Some(0x0800_0000 | 0x0100_0030));
        let b = Binding::parse("<Control>space").unwrap();
        assert_eq!(b.qt_combined(), Some(0x0400_0000 | 0x20));
        let b = Binding::parse("<Control>é").unwrap();
        assert_eq!(b.qt_combined(), None);
    }

    #[test]
    fn detect_order_and_lists() {
        assert_eq!(detect_mechanism("GNOME"), Mechanism::Gnome);
        assert_eq!(detect_mechanism("ubuntu:GNOME"), Mechanism::Gnome);
        assert_eq!(detect_mechanism("Unity:Unity7:ubuntu"), Mechanism::Gnome);
        assert_eq!(detect_mechanism("GNOME-Flashback"), Mechanism::Gnome);
        assert_eq!(detect_mechanism("Budgie:GNOME"), Mechanism::Gnome);
        assert_eq!(detect_mechanism("KDE"), Mechanism::Kde);
        assert_eq!(detect_mechanism("KDE:GNOME"), Mechanism::Gnome); // first in support order
        assert_eq!(detect_mechanism("X-Cinnamon"), Mechanism::Cinnamon);
        assert_eq!(detect_mechanism("XFCE"), Mechanism::Xfce);
        assert_eq!(detect_mechanism("sway"), Mechanism::Manual);
        assert_eq!(detect_mechanism("Hyprland"), Mechanism::Manual);
        assert_eq!(detect_mechanism(""), Mechanism::Manual);
    }

    #[test]
    fn gvariant_lists() {
        assert_eq!(
            parse_gvariant_list("['/org/a/', '/org/b/']"),
            vec!["/org/a/", "/org/b/"]
        );
        assert!(parse_gvariant_list("@as []").is_empty());
        assert!(parse_gvariant_list("[]").is_empty());
        assert_eq!(parse_gvariant_list("['custom0', 'custom1']"), vec!["custom0", "custom1"]);
        assert_eq!(format_gvariant_list(&[]), "[]");
        assert_eq!(
            format_gvariant_list(&["a".to_string()]),
            "['a']"
        );
    }

    #[test]
    fn kde_config_parse() {
        // KF5 flat group with triplet value
        let kf5 = "[activity]\n_lastList=none\n\n[net.local.cleepboard-toggle.desktop]\n_launch=Ctrl+Shift+V,none,Cleepboard Toggle\n";
        assert_eq!(kde_launch_value(kf5), Some("Ctrl+Shift+V".to_string()));
        // KF6 nested [services] group
        let kf6 = "[services][net.local.cleepboard-toggle.desktop]\n_launch=Ctrl+Shift+V,,\n";
        assert_eq!(kde_launch_value(kf6), Some("Ctrl+Shift+V".to_string()));
        // explicitly unbound / missing
        let un = "[net.local.cleepboard-toggle.desktop]\n_launch=none,none,\n";
        assert_eq!(kde_launch_value(un), None);
        assert_eq!(kde_launch_value("[other]\n_launch=x\n"), None);
    }

    #[test]
    fn kde_strip_sections() {
        let content = "[keep]\nk=v\n\n[net.local.cleepboard-toggle.desktop]\n_launch=x,,\n\n[services][net.local.cleepboard-toggle.desktop]\n_launch=y,,\n";
        let out = strip_ini_sections(
            content,
            &[
                format!("[{}]", KDE_COMPONENT),
                format!("[services][{}]", KDE_COMPONENT),
            ],
        );
        assert_eq!(out, "[keep]\nk=v\n\n");
    }

    #[test]
    fn xfce_list_parse() {
        let out = "/commands/custom/<Primary><Alt>t   xfce4-terminal\n/commands/custom/<Primary><Shift>v   /home/u/bin/cleepboard --toggle\n/properties/else   ignored\n";
        let props = xfce_props(out, "/home/u/bin/cleepboard --toggle");
        assert_eq!(props, vec!["/commands/custom/<Primary><Shift>v"]);
        // stale command path still matches via cleepboard+--toggle
        let out2 = "/commands/custom/<Primary>x   /usr/bin/cleepboard --toggle\n";
        assert_eq!(xfce_props(out2, "/home/u/bin/cleepboard --toggle").len(), 1);
        // unrelated command ignored
        let out3 = "/commands/custom/<Primary>t   thunar\n";
        assert!(xfce_props(out3, "/home/u/bin/cleepboard --toggle").is_empty());
    }

    #[test]
    fn mechanism_names() {
        assert_eq!(Mechanism::Kde.name(), "kde");
        assert_eq!(Mechanism::Manual.name(), "manual");
    }

    #[test]
    fn instructions_are_per_de() {
        assert!(manual_instructions("Hyprland", "/usr/bin/cleepboard").contains("hyprland.conf"));
        assert!(manual_instructions("sway", "/usr/bin/cleepboard").contains("bindsym"));
        assert!(manual_instructions("LXQt", "/usr/bin/cleepboard").contains("Shortcut Keys"));
        assert!(manual_instructions("mystery", "/usr/bin/cleepboard").contains("keyboard settings"));
    }
}
