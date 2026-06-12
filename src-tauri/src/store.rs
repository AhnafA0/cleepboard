use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: String,
    pub kind: String, // "text" | "image"
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub preview: String,
    pub pinned: bool,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub max_history: usize,
    pub auto_paste: bool,
    pub theme: String, // "system" | "light" | "dark"
    pub poll_ms: u64,
    pub launch_on_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_history: 100,
            auto_paste: true,
            theme: "system".into(),
            poll_ms: 700,
            launch_on_login: false,
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
    pub fn add_text(&mut self, text: String) -> bool {
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
        };
        self.insert_respecting_pins(item);
        self.trim();
        self.save_history();
        true
    }

    /// Add an image clip from raw PNG bytes. Returns true if newly added.
    pub fn add_image(&mut self, bytes: &[u8]) -> bool {
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
