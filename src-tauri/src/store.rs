use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: String,
    pub kind: String, // "text" | "image" | "file"
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub preview: String,
    pub pinned: bool,
    pub timestamp: u64,
    // Name of the app that owned the clipboard selection at copy time. Captured
    // on X11 via `xdotool getactivewindow getwindowname`; `None` on Wayland
    // (no reliable portal-free way to read the focused app from a background
    // process). `#[serde(default)]` keeps old `history.json` files loadable.
    #[serde(default)]
    pub source_app: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub max_history: usize,
    pub auto_paste: bool,
    pub theme: String, // "system" | "light" | "dark"
    pub poll_ms: u64,
    pub launch_on_login: bool,
    pub data_dir: String, // empty = use default config dir
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_history: 100,
            auto_paste: true,
            theme: "system".into(),
            poll_ms: 700,
            launch_on_login: false,
            data_dir: String::new(),
        }
    }
}

pub struct Store {
    pub items: Vec<ClipItem>,
    pub settings: Settings,
    dir: PathBuf,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Store {
    pub fn load(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(dir.join("images"));

        let settings = fs::read_to_string(dir.join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let items: Vec<ClipItem> = fs::read_to_string(dir.join("history.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Store { items, settings, dir }
    }

    pub fn images_dir(&self) -> PathBuf {
        self.dir.join("images")
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Migrate all data (history, images, settings) to a new directory.
    pub fn migrate_to(&self, new_dir: &Path) -> Result<Store, String> {
        fs::create_dir_all(new_dir)
            .map_err(|e| format!("Cannot create directory: {}", e))?;
        fs::create_dir_all(new_dir.join("images"))
            .map_err(|e| format!("Cannot create images directory: {}", e))?;

        let history_src = self.dir.join("history.json");
        let history_dst = new_dir.join("history.json");
        if history_src.exists() {
            fs::copy(&history_src, &history_dst)
                .map_err(|e| format!("Cannot copy history: {}", e))?;
        }

        let images_src = self.images_dir();
        let images_dst = new_dir.join("images");
        if images_src.exists() {
            for entry in fs::read_dir(&images_src).map_err(|e| format!("Cannot read images: {}", e))? {
                let entry = entry.map_err(|e| format!("Cannot read image entry: {}", e))?;
                let src = entry.path();
                let Some(fname) = src.file_name() else { continue };
                let dst = images_dst.join(fname);
                fs::copy(&src, &dst).map_err(|e| format!("Cannot copy image: {}", e))?;
            }
        }

        // Rewrite image paths in copied history so they point to the new directory.
        if history_dst.exists() {
            let history_str = fs::read_to_string(&history_dst)
                .map_err(|e| format!("Cannot read copied history: {}", e))?;
            let mut items: Vec<ClipItem> = serde_json::from_str(&history_str)
                .map_err(|e| format!("Cannot parse copied history: {}", e))?;
            for item in &mut items {
                if let Some(ref mut path) = item.image_path {
                    if let Some(fname) = Path::new(path).file_name() {
                        *path = images_dst.join(fname).to_string_lossy().to_string();
                    }
                }
            }
            let fixed = serde_json::to_string_pretty(&items)
                .map_err(|e| format!("Cannot serialize fixed history: {}", e))?;
            fs::write(&history_dst, fixed)
                .map_err(|e| format!("Cannot write fixed history: {}", e))?;
        }

        let mut new_settings = self.settings.clone();
        new_settings.data_dir = new_dir.to_string_lossy().to_string();
        let settings_dst = new_dir.join("settings.json");
        let s = serde_json::to_string_pretty(&new_settings)
            .map_err(|e| format!("Cannot serialize settings: {}", e))?;
        fs::write(&settings_dst, s)
            .map_err(|e| format!("Cannot write settings: {}", e))?;

        Ok(Store::load(new_dir.to_path_buf()))
    }

    pub fn save_history(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.items) {
            let _ = fs::write(self.dir.join("history.json"), s);
        }
    }

    pub fn save_settings(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write(self.dir.join("settings.json"), s);
        }
    }

    /// Add a text clip. Returns true if it was newly added.
    pub fn add_text(&mut self, text: String, source_app: Option<String>) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }
        // De-dupe: if identical text already exists, move it to top (or keep pinned position).
        if let Some(pos) = self
            .items
            .iter()
            .position(|i| i.kind == "text" && i.text.as_deref() == Some(text.as_str()))
        {
            let mut existing = self.items.remove(pos);
            existing.timestamp = now_secs();
            existing.source_app = source_app;
            self.insert_respecting_pins(existing);
            self.save_history();
            return false;
        }
        let preview: String = text.chars().take(160).collect();
        let item = ClipItem {
            id: gen_id(),
            kind: "text".into(),
            text: Some(text),
            image_path: None,
            preview,
            pinned: false,
            timestamp: now_secs(),
            source_app,
        };
        self.insert_respecting_pins(item);
        self.trim();
        self.save_history();
        true
    }

    /// Add an image clip from raw PNG bytes. Returns true if newly added.
    pub fn add_image(&mut self, bytes: &[u8], source_app: Option<String>) -> bool {
        if bytes.is_empty() {
            return false;
        }
        let sig = simple_hash(bytes);
        // De-dupe by stored signature embedded in filename.
        if let Some(pos) = self.items.iter().position(|i| {
            i.kind == "image"
                && i.image_path
                    .as_deref()
                    .map(|p| p.contains(&sig))
                    .unwrap_or(false)
        }) {
            let mut existing = self.items.remove(pos);
            existing.timestamp = now_secs();
            existing.source_app = source_app;
            self.insert_respecting_pins(existing);
            self.save_history();
            return false;
        }
        let fname = format!("img_{}_{}.png", now_secs(), sig);
        let path = self.images_dir().join(&fname);
        if fs::write(&path, bytes).is_err() {
            return false;
        }
        let item = ClipItem {
            id: gen_id(),
            kind: "image".into(),
            text: None,
            image_path: Some(path.to_string_lossy().to_string()),
            preview: if bytes.len() >= 1024 {
                format!("Image · {} KB", bytes.len() / 1024)
            } else {
                format!("Image · {} B", bytes.len())
            },
            pinned: false,
            timestamp: now_secs(),
            source_app,
        };
        self.insert_respecting_pins(item);
        self.trim();
        self.save_history();
        true
    }

    /// Add a file clip from a list of `file://` URIs (one copy action = one
    /// clip, even for multiple files). The raw `text/uri-list` payload is
    /// stored in `text` so `copy_item` can re-copy it via `set_files`; the
    /// preview is the comma-joined filenames. Returns true if newly added.
    pub fn add_file(&mut self, uris: Vec<String>, source_app: Option<String>) -> bool {
        if uris.is_empty() {
            return false;
        }
        let payload = uris.join("\n");
        // De-dupe by the stored URI-list payload (same pattern as text).
        if let Some(pos) = self
            .items
            .iter()
            .position(|i| i.kind == "file" && i.text.as_deref() == Some(payload.as_str()))
        {
            let mut existing = self.items.remove(pos);
            existing.timestamp = now_secs();
            existing.source_app = source_app;
            self.insert_respecting_pins(existing);
            self.save_history();
            return false;
        }
        let preview: String = uris
            .iter()
            .map(|u| uri_file_name(u))
            .collect::<Vec<_>>()
            .join(", ");
        let item = ClipItem {
            id: gen_id(),
            kind: "file".into(),
            text: Some(payload),
            image_path: None,
            preview,
            pinned: false,
            timestamp: now_secs(),
            source_app,
        };
        self.insert_respecting_pins(item);
        self.trim();
        self.save_history();
        true
    }

    fn insert_respecting_pins(&mut self, item: ClipItem) {
        // Pinned items stay at the front. New/un-pinned items go right after the
        // last pinned item so pins are always on top.
        if item.pinned {
            self.items.insert(0, item);
            return;
        }
        let insert_at = self.items.iter().take_while(|i| i.pinned).count();
        self.items.insert(insert_at, item);
    }

    fn trim(&mut self) {
        let max = self.settings.max_history.max(1);
        while self.items.iter().filter(|i| !i.pinned).count() > max {
            // Remove the oldest non-pinned item (last in list).
            if let Some(pos) = self.items.iter().rposition(|i| !i.pinned) {
                let removed = self.items.remove(pos);
                self.cleanup_image(&removed);
            } else {
                break;
            }
        }
    }

    fn cleanup_image(&self, item: &ClipItem) {
        if let Some(p) = &item.image_path {
            let _ = fs::remove_file(p);
        }
    }

    pub fn find(&self, id: &str) -> Option<&ClipItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn toggle_pin(&mut self, id: &str) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            let mut item = self.items.remove(pos);
            item.pinned = !item.pinned;
            self.insert_respecting_pins(item);
            self.save_history();
        }
    }

    pub fn delete(&mut self, id: &str) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            let removed = self.items.remove(pos);
            self.cleanup_image(&removed);
            self.save_history();
        }
    }

    pub fn clear(&mut self, keep_pinned: bool) {
        let removed: Vec<ClipItem> = if keep_pinned {
            let (keep, drop): (Vec<_>, Vec<_>) =
                self.items.drain(..).partition(|i| i.pinned);
            self.items = keep;
            drop
        } else {
            self.items.drain(..).collect()
        };
        for item in &removed {
            self.cleanup_image(item);
        }
        self.save_history();
    }
}

fn gen_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

/// Best-effort filename extraction from a `file://` URI for the clip preview.
/// Falls back to the raw URI if parsing fails.
fn uri_file_name(uri: &str) -> String {
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    // Decode percent-encoded sequences we commonly care about (%20 etc.).
    let decoded = percent_decode(path);
    Path::new(&decoded)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| decoded.clone())
}

/// Percent-decode a `file://` URI path component. Decoded bytes (both from
/// `%XX` sequences and any literal bytes) are accumulated and reassembled as
/// UTF-8 at the end, so percent-encoded multi-byte chars (`caf%C3%A9.txt` →
/// `café.txt`) and raw UTF-8 (`café.txt`) round-trip correctly. Decoding
/// byte-by-byte into `char` would interpret each byte as Latin-1 and corrupt
/// any non-ASCII filename.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Tiny non-cryptographic hash (FNV-1a) used only for de-duplicating images.
fn simple_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:x}", hash)
}

#[allow(dead_code)]
pub fn config_dir_for(app_name: &str) -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Path::new(&xdg).join(app_name);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home).join(".config").join(app_name);
    }
    PathBuf::from(".").join(app_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_ascii_and_spaces() {
        assert_eq!(percent_decode("hello%20world.txt"), "hello world.txt");
        assert_eq!(percent_decode("no-encoding"), "no-encoding");
        assert_eq!(percent_decode("a%2Bb"), "a+b");
    }

    #[test]
    fn percent_decode_utf8_encoded_multibyte() {
        // "café" UTF-8 is 63 61 66 C3 A9; file managers emit %C3%A9.
        assert_eq!(percent_decode("caf%C3%A9.txt"), "café.txt");
        // CJK: "名" = E5 90 8D, "前" = E5 89 8D.
        assert_eq!(percent_decode("%E5%90%8D%E5%89%8D.txt"), "名前.txt");
    }

    #[test]
    fn percent_decode_raw_utf8_bytes_preserved() {
        // A URI that already contains literal UTF-8 (not percent-encoded) must
        // survive intact rather than being split into Latin-1 chars.
        assert_eq!(percent_decode("café.txt"), "café.txt");
    }

    #[test]
    fn percent_decode_invalid_utf8_is_lossy() {
        // Lone continuation byte 0xA9 is not valid UTF-8 on its own; the
        // lossy decoder replaces it with U+FFFD instead of panicking.
        assert_eq!(percent_decode("%A9"), "\u{FFFD}");
    }

    #[test]
    fn percent_decode_truncated_escape_left_alone() {
        // A trailing "%" with no hex digits must pass through unchanged.
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("file%2"), "file%2");
    }

    #[test]
    fn uri_file_name_decodes_and_extracts() {
        assert_eq!(uri_file_name("file:///home/user/caf%C3%A9.txt"), "café.txt");
        assert_eq!(uri_file_name("file:///tmp/report.final.pdf"), "report.final.pdf");
        // No file:// prefix and no path separators: fall back to the decoded value.
        assert_eq!(uri_file_name("plain%20name"), "plain name");
    }
}
