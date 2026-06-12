use std::process::Command;

const KB_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/cleepboard/";
const SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const CUSTOM_SCHEMA: &str =
    "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";

fn gsettings(args: &[&str]) -> Option<String> {
    let out = Command::new("gsettings").args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn has_gsettings() -> bool {
    super::clipboard::which("gsettings")
}

/// Register a GNOME custom keyboard shortcut that runs `<exe> --toggle`.
/// Returns a human-readable status string.
pub fn register(binding: &str) -> Result<String, String> {
    if !has_gsettings() {
        return Err("gsettings not found (not a GNOME session)".into());
    }
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();
    let command = format!("{} --toggle", exe);

    // Read the current list of custom keybinding paths.
    let current =
        gsettings(&["get", SCHEMA, "custom-keybindings"]).unwrap_or_else(|| "@as []".into());

    let mut paths: Vec<String> = parse_gvariant_list(&current);
    if !paths.iter().any(|p| p == KB_PATH) {
        paths.push(KB_PATH.to_string());
    }
    let new_list = format_gvariant_list(&paths);
    gsettings(&["set", SCHEMA, "custom-keybindings", &new_list])
        .ok_or("failed to set custom-keybindings list")?;

    let rel = format!("{}:{}", CUSTOM_SCHEMA, KB_PATH);
    gsettings(&["set", &rel, "name", "Cleepboard Toggle"])
        .ok_or("failed to set name")?;
    gsettings(&["set", &rel, "command", &command])
        .ok_or("failed to set command")?;
    gsettings(&["set", &rel, "binding", binding])
        .ok_or("failed to set binding")?;

    Ok(format!("Registered {} -> {}", binding, command))
}

pub fn unregister() -> Result<String, String> {
    if !has_gsettings() {
        return Err("gsettings not found".into());
    }
    let current =
        gsettings(&["get", SCHEMA, "custom-keybindings"]).unwrap_or_else(|| "@as []".into());
    let paths: Vec<String> = parse_gvariant_list(&current)
        .into_iter()
        .filter(|p| p != KB_PATH)
        .collect();
    let new_list = format_gvariant_list(&paths);
    gsettings(&["set", SCHEMA, "custom-keybindings", &new_list])
        .ok_or("failed to update list")?;
    Ok("Unregistered".into())
}

/// Whether our keybinding is currently registered.
pub fn is_registered() -> bool {
    if !has_gsettings() {
        return false;
    }
    let current = gsettings(&["get", SCHEMA, "custom-keybindings"]).unwrap_or_default();
    parse_gvariant_list(&current).iter().any(|p| p == KB_PATH)
}

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
