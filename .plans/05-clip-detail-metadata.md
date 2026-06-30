# Plan 05 — Clip Detail metadata (timestamp + source app)

## Goal
The Clip Detail modal currently shows only the raw content in a `<pre>`. The
design's `NoteDetail` specifies metadata (timestamp, source app) alongside the
full preview.

## Design reference (`design.pen`)
- Screen: `Screen/ClipDetail` (`I0lwcw`)
  - `DetailHeader` (`HOpkj`) with title + close
  - `DetailBody` (`aMoaU`) with `DetailPreviewText` (`FohOV`)
  - `DetailFooter` (`I1YJL`) with `DetailCopyBtn` (`C8o13x`)
- Annotation `NoteDetail`:
  > Clip Detail/Preview: Expanded view for images or long text. Shows full
  > content without truncation. Copy button to re-copy. Metadata: timestamp,
  > source app. Dismiss: Esc or close button.

## Files to touch
- `src-tauri/src/clipboard.rs` — capture the source app (Wayland/X11)
- `src-tauri/src/store.rs` — add `source_app: Option<String>` to `ClipItem`
- `src-tauri/src/lib.rs` — pass source through to `add_text`/`add_image`
- `src/index.html` — add a metadata strip to the detail dialog body
- `src/main.js` — render metadata in `openDetail()`
- `src/styles.css` — `.detail-meta` styling

## Implementation steps

### 1. Capture the source app (the hard part — platform research needed first)
On **X11**: the focused window at copy time owns the clipboard selection;
`xdotool getactivewindow getwindowname` gives the active window title.
On **Wayland**: there is no reliable portal-free way to get the focused app
from a background process. Options:
  - `gdbus` call to `org.gnome.Shell` / `org.kde.KWin` (GNOME-specific,
    brittle across versions).
  - Read `/proc/<pid>` of `wl-paste`'s parent — won't give the *copying* app.
  - **Recommendation:** implement source-app capture for X11 only (xdotool),
    and leave it `None` on Wayland with a graceful "Unknown" display. Document
    this limitation in the detail view ("Source app tracking unavailable on
    Wayland"). This keeps the feature useful on X11 without fragile GNOME
    shell introspection.

Capture flow: in `spawn_watcher` (lib.rs:216), right before reading the
clipboard, snapshot the active window name (X11) into a local var, then pass
it into `add_text`/`add_image`.

### 2. Store model
Add to `ClipItem` (store.rs:7):
```rust
pub source_app: Option<String>,
```
Update both `add_text` and `add_image` signatures to accept
`source_app: Option<String>` and store it. Bump any history.json that loads
without the field — serde `#[serde(default)]` on the field handles old files.

### 3. Frontend — HTML
Add a metadata strip at the top of `#detail-body`:
```html
<div id="detail-meta" class="detail-meta"></div>
```
(above the existing `<pre>`/`<img>` content that `openDetail` injects).

### 4. Frontend — JS (`openDetail`, src/main.js:195)
```js
const meta = $("#detail-meta");
const time = new Date(item.timestamp * 1000).toLocaleString();
const src = item.source_app ? escapeHtml(item.source_app) : "Unknown";
meta.innerHTML = `<span>Copied ${time}</span><span>·</span><span>From ${src}</span>`;
```

### 5. CSS
```css
.detail-meta {
  display: flex; gap: var(--space-sm); align-items: center;
  font-size: var(--text-sm); color: var(--text-muted);
  padding-bottom: var(--space-md); border-bottom: 1px solid var(--border);
  margin-bottom: var(--space-md);
}
```

## Verification
1. X11 session: copy text from Firefox → detail shows "From Firefox".
2. Wayland session: detail shows "From Unknown" (expected limitation).
3. Old `history.json` without `source_app` loads without error (serde default).
4. Image clips also show timestamp + source.
5. Esc and close button still dismiss the dialog.
