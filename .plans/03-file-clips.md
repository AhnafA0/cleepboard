# Plan 03 — File-clip variant

## Goal
Add support for clipboard file copies (e.g. copying a file in Nautilus) as a
distinct `kind: "file"` clip, with the `Component/ClipCard/File` card variant
from the design. Currently the store only models `text` and `image`.

## Design reference (`design.pen`)
- Component: `Component/ClipCard/File` (`uS2P1`)
  - `ClipFileIcon` (`boZ4b`) — a framed icon showing the file extension text
  - `ClipContentFrame` (`Dmsz1`) with `ClipText` + `ClipMeta`
- Filter chip **Files** (Plan 01) filters `kind === "file"`.
- Annotation `NoteHistoryUpdates` lists Files as one of the filter categories.

The file card shows a file-type badge (extension in a small framed box) instead
of the round accent icon used for text clips.

## Files to touch
- `src-tauri/src/clipboard.rs` — detect file URIs in the clipboard
- `src-tauri/src/store.rs` — `add_file()` method, `ClipItem` already has `kind`
- `src-tauri/src/lib.rs` — watcher branch for file content
- `src/main.js` — `iconFor()` file branch, render file extension badge
- `src/styles.css` — `.clip-file-icon` styling

## Implementation steps

### 1. Detect file copies (`clipboard.rs`)
On Wayland, `wl-paste --list-types` includes `text/uri-list` (or
`x-special/gnome-copied-files`) when files are copied. On X11, `xclip -o -t
text/uri-list` returns the URI list.

Add a `ClipContent::Files(Vec<PathBuf>)` variant (or `Files(Vec<String>)` of
URIs) to the `read()` function:
- Probe for `text/uri-list` first; parse `file://` URIs into paths.
- Fall back to text/image as today if no URI list is present.

Signature for de-dup: `f:<comma-joined-paths>`.

### 2. Store (`store.rs`)
Add:
```rust
pub fn add_file(&mut self, paths: Vec<String>) -> bool {
    if paths.is_empty() { return false; }
    let joined = paths.join(", ");
    // de-dupe by joined path string (same as text de-dupe pattern)
    let preview = paths.iter()
        .map(|p| Path::new(p).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default())
        .collect::<Vec<_>>().join(", ");
    // store the joined URI list in `text` so copy_item can re-copy it,
    // and set kind = "file". Optionally store original paths separately.
    ...
}
```
Decide: store the raw `text/uri-list` payload in `ClipItem.text` so re-copy
works via `clipboard::set_text` with the `text/uri-list` mime type, OR add a
dedicated `file_uris: Option<Vec<String>>` field. The simpler path is reusing
`text` + a `kind: "file"` discriminator, but **re-copying files requires
writing the `text/uri-list` mime type**, not plain text — see step 4.

### 3. Watcher (`lib.rs` `spawn_watcher`)
Add a match arm for `ClipContent::Files(paths)` → `store.add_file(paths)`,
mirroring the existing Text/Image arms (src-tauri/src/lib.rs:251-258).
Update `sig_image`/`sig_text` pattern to add `sig_file`.

### 4. Re-copy (`copy_item` in `lib.rs`)
For `kind == "file"`, write the stored URI list back to the clipboard as
`text/uri-list` (Wayland: `wl-copy --type text/uri-list`; X11:
`xclip -i -t text/uri-list`). This needs a new `clipboard::set_files()` helper.

### 5. Frontend (`main.js`)
In `iconFor()` (src/main.js:35) add a file branch BEFORE the text branches:
```js
if (item.kind === "file") {
  const ext = (item.preview.match(/\.(\w+)$/) || [,""])[1].toUpperCase().slice(0,4) || "FILE";
  return `<span class="clip-file-ext">${ext}</span>`;
}
```
Render: the `.clip-icon` for file clips should show the framed extension badge
from the design rather than an SVG.

### 6. CSS
```css
.clip-icon .clip-file-ext {
  font-size: 9px; font-weight: 700; color: var(--accent);
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-radius: var(--radius-sm); padding: 2px 4px; line-height: 1;
}
```

## Open questions to resolve while implementing
- **Multiple files**: when the user copies 3 files, is that one clip with 3
  paths or 3 clips? Design shows a single card → **one clip, comma-joined
  filenames in the preview**.
- **Image-as-file**: Nautilus copying a PNG may offer both `image/png` and
  `text/uri-list`. Prefer the `text/uri-list` (file) representation only when
  the source is a file manager, not an image editor. Heuristic: if
  `image/png` is offered AND `text/uri-list` is offered, treat as image
  (richer preview). If ONLY `text/uri-list`, treat as file. Document this.

## Verification
1. In Nautilus, copy a file → it appears as a file clip with the extension
   badge and the filename in the preview.
2. Click the clip → the file is re-copied to the clipboard (paste in Nautilus
   works).
3. The **Files** filter chip (Plan 01) now shows file clips.
4. De-dup: copying the same file twice doesn't create a duplicate; it bumps
   the existing clip to the top.
5. Copying 3 files at once → one clip, preview "a.txt, b.md, c.rs".
