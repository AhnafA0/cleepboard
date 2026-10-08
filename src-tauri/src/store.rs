use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Largest text payload stored per clip. Bigger copies are skipped outright:
/// a multi-megabyte text would be re-serialized into history.json on every
/// mutation and shipped over IPC to every render.
const MAX_TEXT_BYTES: usize = 256 * 1024;
/// Largest image payload stored per clip.
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
/// Text search covers the preview plus this leading slice of a clip's full
/// payload — deep enough for real use without lowering megabytes per query.
const SEARCH_TEXT_CAP: usize = 64 * 1024;
/// Longest edge of a stored thumbnail, in pixels.
const THUMB_MAX_DIM: u32 = 96;
/// Bytes of an image payload hashed for the watcher's signature. The prefix
/// is enough to distinguish images; hashing a multi-MB PNG whole on every
/// poll is what the cap avoids.
const IMAGE_SIG_PREFIX: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
    // process).
    pub source_app: Option<String>,
}

impl Default for ClipItem {
    fn default() -> Self {
        ClipItem {
            id: String::new(),
            kind: "text".into(),
            text: None,
            image_path: None,
            preview: String::new(),
            pinned: false,
            timestamp: 0,
            source_app: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub max_history: usize,
    pub auto_paste: bool,
    pub theme: String, // "system" | "light" | "dark"
    pub poll_ms: u64,
    pub data_dir: String, // empty = use default config dir
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_history: 100,
            auto_paste: true,
            theme: "system".into(),
            poll_ms: 700,
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

/// Write `contents` to `path` atomically: tmp file in the same directory,
/// then rename. A crash mid-write can then only ever lose the *new* file —
/// the previous history.json/settings.json survives intact.
fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}

/// First `max_bytes` of `s`, not splitting a UTF-8 char boundary.
fn head_slice(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

impl Store {
    pub fn load(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(dir.join("images"));

        let settings = fs::read_to_string(dir.join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let history_path = dir.join("history.json");
        let items: Vec<ClipItem> = match fs::read_to_string(&history_path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(items) => items,
                // Corrupt history (crash mid-write predating atomic saves, or
                // hand-edit): move it aside instead of letting the next save
                // silently wipe the only copy of the user's clips.
                Err(_) => {
                    let bak = dir.join(format!("history.corrupt-{}.json", now_secs()));
                    let _ = fs::rename(&history_path, &bak);
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        };

        Store { items, settings, dir }
    }

    pub fn images_dir(&self) -> PathBuf {
        self.dir.join("images")
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Migrate all data (history, images, settings) to a new directory.
    /// `new_dir` must be canonicalized already and provably different from
    /// the current dir — `set_data_dir` enforces that. On success the caller
    /// follows with `purge_data` on the old store.
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
            write_atomic(&history_dst, &fixed)
                .map_err(|e| format!("Cannot write fixed history: {}", e))?;
        }

        let mut new_settings = self.settings.clone();
        new_settings.data_dir = new_dir.to_string_lossy().to_string();
        let settings_dst = new_dir.join("settings.json");
        let s = serde_json::to_string_pretty(&new_settings)
            .map_err(|e| format!("Cannot serialize settings: {}", e))?;
        write_atomic(&settings_dst, &s)
            .map_err(|e| format!("Cannot write settings: {}", e))?;

        Ok(Store::load(new_dir.to_path_buf()))
    }

    /// Remove the data artifacts this store owns (history.json + images/)
    /// after a successful migrate_to, so the old location doesn't keep a
    /// stale copy of everything. The directory itself and settings.json are
    /// left alone — the dir may be the shared config dir, and "Move" is not
    /// "wipe the folder".
    pub fn purge_data(&self) {
        let _ = fs::remove_file(self.dir.join("history.json"));
        let _ = fs::remove_dir_all(self.images_dir());
    }

    pub fn save_history(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.items) {
            let _ = write_atomic(&self.dir.join("history.json"), &s);
        }
    }

    pub fn save_settings(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.settings) {
            let _ = write_atomic(&self.dir.join("settings.json"), &s);
        }
    }

    /// A copy of each item with the bulky fields (`text`, `image_path`)
    /// stripped — what the UI list needs. Full text is fetched on demand via
    /// `get_item_text`; images via `get_image_thumb` / `copy_item`'s own lookup.
    pub fn summaries(&self) -> Vec<ClipItem> {
        self.items.iter().map(stripped).collect()
    }

    /// Case-insensitive substring match over the preview and the first
    /// SEARCH_TEXT_CAP bytes of a text clip's payload. Returns stripped items.
    pub fn search(&self, query: &str) -> Vec<ClipItem> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.summaries();
        }
        self.items
            .iter()
            .filter(|i| {
                i.preview.to_lowercase().contains(&q)
                    || i.text
                        .as_deref()
                        .map(|t| head_slice(t, SEARCH_TEXT_CAP).to_lowercase().contains(&q))
                        .unwrap_or(false)
            })
            .map(stripped)
            .collect()
    }

    /// Add a text clip. Returns true whenever history changed (new clip, or
    /// an existing identical clip bumped to the top).
    pub fn add_text(&mut self, text: String, source_app: Option<String>) -> bool {
        if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
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
            return true;
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

    /// Add an image clip from raw PNG bytes. Returns true if history changed.
    /// Also writes a bounded thumbnail used by the list view so renders don't
    /// base64 the full payload per clip.
    pub fn add_image(&mut self, bytes: &[u8], source_app: Option<String>) -> bool {
        if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
            return false;
        }
        let sig = simple_hash(bytes);
        // De-dupe by stored signature embedded in filename. The match is
        // anchored to the filename's `_{sig}.png` tail so an all-digit sig
        // can't collide with the timestamp segment.
        if let Some(pos) = self.items.iter().position(|i| {
            i.kind == "image"
                && i.image_path
                    .as_deref()
                    .map(|p| image_sig_matches(p, &sig))
                    .unwrap_or(false)
        }) {
            let mut existing = self.items.remove(pos);
            existing.timestamp = now_secs();
            existing.source_app = source_app;
            self.insert_respecting_pins(existing);
            self.save_history();
            return true;
        }
        let fname = format!("img_{}_{}.png", now_secs(), sig);
        let path = self.images_dir().join(&fname);
        if fs::write(&path, bytes).is_err() {
            return false;
        }
        // Thumbnail alongside the full image; failure just means the UI falls
        // back to the full-size data URL.
        let thumb_path = self.images_dir().join(format!("thumb_{}", fname));
        let _ = write_thumbnail(bytes, &thumb_path);

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
    /// preview is the comma-joined filenames. Returns true if history changed.
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
            return true;
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

    /// The stored image file for `item`, verified to live directly inside our
    /// images dir. `image_path` comes from history.json — a hand-edited file
    /// must not turn into arbitrary-file read/delete via the IPC commands.
    pub fn image_file(&self, item: &ClipItem) -> Option<PathBuf> {
        let path = item.image_path.as_ref()?;
        let canon = Path::new(path).canonicalize().ok()?;
        let images = self.images_dir().canonicalize().ok()?;
        (canon.parent() == Some(images.as_path())).then_some(canon)
    }

    /// The thumbnail file for `item` (same containment check as `image_file`).
    pub fn thumb_file(&self, item: &ClipItem) -> Option<PathBuf> {
        let img = self.image_file(item)?;
        let fname = format!("thumb_{}", img.file_name()?.to_str()?);
        let canon = img.parent()?.join(fname).canonicalize().ok()?;
        let images = self.images_dir().canonicalize().ok()?;
        (canon.parent() == Some(images.as_path())).then_some(canon)
    }

    /// Lazily create (or return the path of) the thumbnail for an image that
    /// predates thumbnails. Writes into images/ next to the source.
    pub fn ensure_thumb(&self, item: &ClipItem) -> Option<PathBuf> {
        if let Some(t) = self.thumb_file(item) {
            return Some(t);
        }
        let img = self.image_file(item)?;
        let bytes = fs::read(&img).ok()?;
        let thumb_path = img
            .parent()?
            .join(format!("thumb_{}", img.file_name()?.to_str()?));
        write_thumbnail(&bytes, &thumb_path).ok()?;
        Some(thumb_path)
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
        // Only remove files provably inside our images dir (image_path is
        // user-editable data). Resolve the thumbnail path before deleting the
        // image — `thumb_file` verifies containment via the image itself and
        // would fail on an already-removed file.
        let img = self.image_file(item);
        let thumb = img.as_ref().and_then(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .map(|f| p.with_file_name(format!("thumb_{}", f)))
        });
        if let Some(p) = img {
            let _ = fs::remove_file(&p);
        }
        if let Some(t) = thumb {
            let _ = fs::remove_file(t);
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
            std::mem::take(&mut self.items)
        };
        for item in &removed {
            self.cleanup_image(item);
        }
        self.save_history();
    }
}

/// Clone an item with the heavyweight fields blanked — the shape the UI list
/// and `history-updated` payloads travel in.
fn stripped(item: &ClipItem) -> ClipItem {
    ClipItem {
        text: None,
        image_path: None,
        ..item.clone()
    }
}

/// `file_name` ends with the `_{sig}.png` tail produced by add_image —
/// anchored so an all-digit sig can't match the timestamp segment instead.
fn image_sig_matches(path: &str, sig: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(&format!("_{}.png", sig)))
        .unwrap_or(false)
}

/// Decode `bytes` (any supported image format), resize to THUMB_MAX_DIM on
/// the long edge, and save as PNG at `path`.
fn write_thumbnail(bytes: &[u8], path: &Path) -> std::io::Result<()> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    img.thumbnail(THUMB_MAX_DIM, THUMB_MAX_DIM)
        .save(path)
        .map_err(|e| std::io::Error::other(e.to_string()))
}

/// FNV-1a over the first IMAGE_SIG_PREFIX bytes of an image payload — the
/// signature the watcher and self-set marker use. Bounded so a multi-MB PNG
/// doesn't cost a full read every poll.
pub fn image_sig(bytes: &[u8]) -> String {
    simple_hash(&bytes[..bytes.len().min(IMAGE_SIG_PREFIX)])
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

/// Expand a user-entered data-dir path: `~`/`~/…` against $HOME, bare
/// relative paths against $HOME too (a GUI app's launch cwd is arbitrary —
/// HOME is the only base the user can predict).
pub fn expand_dir(input: &str) -> Result<PathBuf, String> {
    let t = input.trim();
    if t.is_empty() {
        return Err("Directory path cannot be empty".into());
    }
    let home = || -> Result<PathBuf, String> {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| "cannot resolve path: $HOME is not set".to_string())
    };
    let expanded = if t == "~" {
        home()?
    } else if let Some(rest) = t.strip_prefix("~/") {
        home()?.join(rest)
    } else if Path::new(t).is_absolute() {
        PathBuf::from(t)
    } else {
        home()?.join(t)
    };
    Ok(expanded)
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

    #[test]
    fn image_sig_match_is_anchored() {
        // Signature is anchored to the filename tail, so a sig that happens to
        // be all digits can't match the timestamp of an unrelated image.
        assert!(image_sig_matches("/d/images/img_1700000000_abc123.png", "abc123"));
        assert!(!image_sig_matches("/d/images/img_1700000000_deadbeef.png", "7000000000"));
        assert!(!image_sig_matches("/d/images/img_1234_5678.png", "34_5678"));
    }

    #[test]
    fn head_slice_respects_utf8_boundaries() {
        // "aébc" = 5 bytes (é is 2): cutting at byte 2 must not split é.
        assert_eq!(head_slice("aébc", 2), "a");
        assert_eq!(head_slice("aébc", 3), "aé");
        assert_eq!(head_slice("aébc", 4), "aéb");
        assert_eq!(head_slice("aébc", 5), "aébc");
        assert_eq!(head_slice("short", 100), "short");
    }

    #[test]
    fn expand_dir_tilde_and_relative() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_dir("~/clips").unwrap(), Path::new(&home).join("clips"));
        assert_eq!(expand_dir("~").unwrap(), Path::new(&home));
        assert_eq!(expand_dir("rel/dir").unwrap(), Path::new(&home).join("rel/dir"));
        assert_eq!(expand_dir("/abs/dir").unwrap(), Path::new("/abs/dir"));
        assert!(expand_dir("   ").is_err());
    }
}
