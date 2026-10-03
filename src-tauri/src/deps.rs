//! Audit of the external helper binaries cleepboard shells out to. Every
//! clipboard/paste/hotkey path already no-ops safely when its tool is missing;
//! this report lets the UI surface *which* feature is degraded and the
//! per-distro command that fixes it.

use serde::Serialize;

use crate::clipboard::{which, Backend};
use crate::hotkey::{current_mechanism, Mechanism};

/// Per-distro-family install commands for one tool's package.
#[derive(Clone, Debug, Serialize)]
pub struct InstallCmds {
    pub apt: String,
    pub dnf: String,
    pub pacman: String,
    pub zypper: String,
}

/// One probed binary: presence plus what its absence means on this backend.
#[derive(Clone, Debug, Serialize)]
pub struct ToolStatus {
    /// Binary name as probed on PATH.
    pub name: String,
    pub present: bool,
    /// Stable key grouping alternatives that cover the same feature:
    /// "read" | "write" | "read+write" | "paste" | "hotkey".
    pub group: String,
    /// Human description of what the tool is used for.
    pub feature: String,
    /// What breaks when no installed tool covers `feature`.
    pub breaks: String,
    /// Core feature: surfaces in the overlay banner when uncovered.
    pub required: bool,
    /// An installed tool covering this feature (possibly this one);
    /// empty when the feature is uncovered.
    pub covered_by: String,
    /// Extra caveat worth showing, e.g. a required daemon; "" when none.
    pub note: String,
    pub install: InstallCmds,
}

#[derive(Clone, Debug, Serialize)]
pub struct DependencyReport {
    /// "wayland" | "x11"
    pub backend: String,
    pub tools: Vec<ToolStatus>,
}

fn install(apt: &str, dnf: &str, pacman: &str, zypper: &str) -> InstallCmds {
    InstallCmds {
        apt: format!("sudo apt install {apt}"),
        dnf: format!("sudo dnf install {dnf}"),
        pacman: format!("sudo pacman -S {pacman}"),
        zypper: format!("sudo zypper install {zypper}"),
    }
}

/// Same package name across the major families (the common case).
fn install_all(pkg: &str) -> InstallCmds {
    install(pkg, pkg, pkg, pkg)
}

pub fn report(backend: Backend) -> DependencyReport {
    let tools = match backend {
        Backend::Wayland => wayland_tools(),
        Backend::X11 => x11_tools(),
    };
    DependencyReport {
        backend: match backend {
            Backend::Wayland => "wayland",
            Backend::X11 => "x11",
        }
        .to_string(),
        tools,
    }
}

const HOTKEY_BREAKS: &str = "the Ctrl+Shift+V shortcut can't be registered";

/// gsettings row; emitted only on the gsettings-based desktops
/// (GNOME/Cinnamon) — its DE label follows the detected mechanism.
fn gsettings_tool() -> ToolStatus {
    let de = if current_mechanism() == Mechanism::Cinnamon {
        "Cinnamon"
    } else {
        "GNOME"
    };
    let present = which("gsettings");
    ToolStatus {
        name: "gsettings".into(),
        present,
        group: "hotkey".into(),
        feature: format!("global hotkey registration ({de})"),
        breaks: HOTKEY_BREAKS.into(),
        required: false,
        covered_by: if present { "gsettings" } else { "" }.into(),
        note: format!("{de} desktops only"),
        install: install("libglib2.0-bin", "glib2", "glib2", "glib2-tools"),
    }
}

/// KDE reaches kglobalaccel over D-Bus via gdbus (glib) or busctl
/// (systemd) — either one covers the feature.
fn kde_dbus_tools() -> Vec<ToolStatus> {
    let gdbus = which("gdbus");
    let busctl = which("busctl");
    let cover = if gdbus {
        "gdbus"
    } else if busctl {
        "busctl"
    } else {
        ""
    };
    let row = |name: &str, present: bool, install_cmds: InstallCmds| ToolStatus {
        name: name.into(),
        present,
        group: "hotkey".into(),
        feature: "global hotkey registration (KDE)".into(),
        breaks: HOTKEY_BREAKS.into(),
        required: false,
        covered_by: cover.into(),
        note: "KDE Plasma — either gdbus or busctl reaches kglobalaccel".into(),
        install: install_cmds,
    };
    vec![
        row("gdbus", gdbus, install("libglib2.0-bin", "glib2", "glib2", "glib2-tools")),
        row("busctl", busctl, install_all("systemd")),
    ]
}

fn xfconf_tool() -> ToolStatus {
    let present = which("xfconf-query");
    ToolStatus {
        name: "xfconf-query".into(),
        present,
        group: "hotkey".into(),
        feature: "global hotkey registration (XFCE)".into(),
        breaks: HOTKEY_BREAKS.into(),
        required: false,
        covered_by: if present { "xfconf-query" } else { "" }.into(),
        note: "XFCE only".into(),
        install: install_all("xfconf"),
    }
}

/// Hotkey-registration rows for the current desktop's mechanism (shared
/// with hotkey.rs, not re-matched here). Manual desktops get no row — no
/// installable tool would fix them.
fn hotkey_tools() -> Vec<ToolStatus> {
    match current_mechanism() {
        Mechanism::Gnome | Mechanism::Cinnamon => vec![gsettings_tool()],
        Mechanism::Kde => kde_dbus_tools(),
        Mechanism::Xfce => vec![xfconf_tool()],
        Mechanism::Manual => vec![],
    }
}

fn wayland_tools() -> Vec<ToolStatus> {
    let wl_paste = which("wl-paste");
    let xclip = which("xclip");
    let wl_copy = which("wl-copy");
    let wtype = which("wtype");
    let ydotool = which("ydotool");

    // xclip only counts as a write path when an X display (XWayland) is
    // reachable — same condition as `prefer_xclip_for_set` in clipboard.rs.
    let xclip_usable = xclip && std::env::var("DISPLAY").is_ok();
    let write_cover = if xclip_usable {
        "xclip"
    } else if wl_copy {
        "wl-copy"
    } else {
        ""
    };
    let paste_cover = if wtype {
        "wtype"
    } else if ydotool {
        "ydotool"
    } else {
        ""
    };

    let mut tools = vec![
        ToolStatus {
            name: "wl-paste".into(),
            present: wl_paste,
            group: "read".into(),
            feature: "clipboard read".into(),
            breaks: "new clips won't appear in history".into(),
            required: true,
            covered_by: if wl_paste { "wl-paste" } else { "" }.into(),
            note: "".into(),
            install: install_all("wl-clipboard"),
        },
        ToolStatus {
            name: "xclip".into(),
            present: xclip,
            group: "write".into(),
            feature: "clipboard write".into(),
            breaks: "re-copying a clip fails".into(),
            required: true,
            covered_by: write_cover.into(),
            note: if xclip && !xclip_usable {
                "installed but unusable: no X display (XWayland)".into()
            } else {
                "preferred write path — via XWayland, avoids wl-copy's focus popup".into()
            },
            install: install_all("xclip"),
        },
        ToolStatus {
            name: "wl-copy".into(),
            present: wl_copy,
            group: "write".into(),
            feature: "clipboard write".into(),
            breaks: "re-copying a clip fails".into(),
            required: true,
            covered_by: write_cover.into(),
            note: "fallback write path".into(),
            install: install_all("wl-clipboard"),
        },
        ToolStatus {
            name: "wtype".into(),
            present: wtype,
            group: "paste".into(),
            feature: "auto-paste (simulates Ctrl+V)".into(),
            breaks: "selecting a clip won't paste it — press Ctrl+V manually".into(),
            required: false,
            covered_by: paste_cover.into(),
            note: "".into(),
            install: install_all("wtype"),
        },
        ToolStatus {
            name: "ydotool".into(),
            present: ydotool,
            group: "paste".into(),
            feature: "auto-paste (simulates Ctrl+V)".into(),
            breaks: "selecting a clip won't paste it — press Ctrl+V manually".into(),
            required: false,
            covered_by: paste_cover.into(),
            note: "needs the ydotoold daemon + uinput access".into(),
            install: install_all("ydotool"),
        },
    ];
    tools.extend(hotkey_tools());
    tools
}

fn x11_tools() -> Vec<ToolStatus> {
    let xclip = which("xclip");
    let xdotool = which("xdotool");

    let mut tools = vec![
        ToolStatus {
            name: "xclip".into(),
            present: xclip,
            group: "read+write".into(),
            feature: "clipboard read + write".into(),
            breaks: "history stays empty and re-copy fails".into(),
            required: true,
            covered_by: if xclip { "xclip" } else { "" }.into(),
            note: "".into(),
            install: install_all("xclip"),
        },
        ToolStatus {
            name: "xdotool".into(),
            present: xdotool,
            group: "paste".into(),
            feature: "auto-paste, source-app name".into(),
            breaks: "no auto-paste (press Ctrl+V); clip source unknown".into(),
            required: false,
            covered_by: if xdotool { "xdotool" } else { "" }.into(),
            note: "".into(),
            install: install_all("xdotool"),
        },
    ];
    tools.extend(hotkey_tools());
    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_all_uses_pkg_for_every_family() {
        let c = install_all("wl-clipboard");
        assert_eq!(c.apt, "sudo apt install wl-clipboard");
        assert_eq!(c.dnf, "sudo dnf install wl-clipboard");
        assert_eq!(c.pacman, "sudo pacman -S wl-clipboard");
        assert_eq!(c.zypper, "sudo zypper install wl-clipboard");
    }

    #[test]
    fn gsettings_package_differs_per_family() {
        let c = gsettings_tool().install;
        assert!(c.apt.contains("libglib2.0-bin"));
        assert!(c.dnf.contains("glib2"));
        assert!(c.zypper.contains("glib2-tools"));
    }

    #[test]
    fn report_tools_have_consistent_coverage() {
        for t in report(crate::clipboard::detect_backend()).tools {
            // An uncovered feature can only come from a missing tool.
            if t.covered_by.is_empty() {
                assert!(!t.present || t.name == "xclip");
            }
        }
    }
}
