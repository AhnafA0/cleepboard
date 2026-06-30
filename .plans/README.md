# Cleepboard — implementation plan index

Plans derived from the `design.pen` vs. implementation audit. Each file is
self-contained — pick one up and implement it independently. Ordered by
impact; later plans may depend on earlier ones where noted.

| # | Plan | Effort | Depends on | Status |
|---|------|--------|------------|--------|
| 01 | [Filter chips on the History Overlay](01-filter-chips.md) | Small | — | ✅ Done |
| 02 | [Settings sidebar + NavItem + launch-on-login](02-settings-sidebar.md) | Medium | — | — |
| 03 | [File-clip variant](03-file-clips.md) | Medium | 01 (Files chip is already wired there) | ✅ Done |
| 04 | [Emoji search-by-name](04-emoji-search.md) | Small | — | ✅ Done |
| 05 | [Clip Detail metadata (timestamp + source app)](05-clip-detail-metadata.md) | Medium | — (X11-only source-app capture) | ✅ Done |
| 06 | [Tray recent-clips quick access](06-tray-recent-clips.md) | Medium | — | — |
| 07 | [Interaction-state polish (bundle)](07-interaction-state-polish.md) | Small | — | ✅ Done |

## Suggested order
1. ~~**01** (filter chips) — highest-visibility gap, unblocks the Files chip for 03.~~ ✅ Done.
2. ~~**04** (emoji search) — quickest win, fully self-contained.~~ ✅ Done.
3. ~~**07** (polish bundle) — small CSS mostly; do alongside any of the above.~~ ✅ Done.
4. **02** (settings sidebar) — bigger restructure but no cross-dependencies.
5. **03** (file clips) — backend work; the 01 Files chip already waits for it.
6. **06** (tray recent-clips) — backend work, isolated.
7. **05** (clip detail metadata) — backend source-app capture is X11-only and
   the most uncertain; do last or defer the source-app piece. ✅ Done.

## Progress
- ✅ **01 — Filter chips** (2026-06-30): added `All / Text / Images / Files / Links`
  chip row to the History Overlay. `Files` chip is wired but inert until 03 lands.
- ✅ **04 — Emoji search-by-name** (2026-06-30): replaced the flat `EMOJI` string
  array with `{ ch, name }` objects (2-4 lowercase keywords each). `renderEmoji()`
  now filters by name (case-insensitive substring) or exact char match, shows an
  empty-state message on no matches, and sets `title` tooltips with the keyword
  name. Added `.emoji-empty` styling. No new wiring needed — the existing search
  `input` listener already calls `renderEmoji(searchEl.value)` on the emoji tab.
- ✅ **07 — Interaction-state polish** (2026-06-30): shipped the full bundle.
  - **7a** distinct `.clip.focused` ring vs `.clip.selected` fill (keyboard nav
    sets both, mouse hover sets only the fill).
  - **7b** `:focus-visible` accent ring on `.btn`, `.icon-btn`, `.tab`.
  - **7c** `.btn.ghost` variant for parity with `Component/Button/Ghost`.
  - **7d** flanking hairline dividers on `.section-header` via `::before`/`::after`,
    label+count wrapped in a non-flex span.
  - **7e** `radius-xl` reconciled to **12px** in `src/styles.css` (CSS wins as the
    source of truth; `design.pen` variable left at 12).
  - **7f** near-cursor overlay positioning on **X11 only** via `xdotool
    getmouselocation` + `set_position`, clamped to the current monitor; Wayland
    keeps `center: true`. `clipboard::run_capture` made `pub` for reuse.
- ✅ **03 — File-clip variant** (2026-06-30): added a `kind: "file"` clip backed
  by `text/uri-list` detection.
  - **clipboard.rs**: new `ClipContent::Files(Vec<String>)` variant. `read()`
    probes `text/uri-list` (and `x-special/gnome-copied-files`) on both Wayland
    (`wl-paste --list-types`) and X11 (`xclip -t TARGETS`). Priority is
    `image/png` > `text/uri-list` > `text/plain`, so copying an image *file* in
    Nautilus still lands as an image clip (richer thumbnail) — only file-only
    copies become file clips. New `set_files()` writes the payload back with the
    `text/uri-list` mime (`wl-copy --type` / `xclip -t`) so re-copy pastes in
    Nautilus.
  - **store.rs**: `add_file()` stores the raw newline-joined URI payload in
    `ClipItem.text` (so `copy_item` can re-copy it), sets `kind = "file"`, and
    builds the preview from percent-decoded filenames joined by `, ` (one clip
    per copy action, per the design). De-dupe is by the stored payload string.
  - **lib.rs**: `sig_file()` for the watcher's change detection + self-set
    suppression; watcher dispatches `Files` → `add_file`; `copy_item` has a new
    `"file"` arm calling `set_files`.
  - **main.js**: `iconFor()` renders a framed `.clip-file-ext` badge (first
    filename's extension, uppercased, max 4 chars, fallback `FILE`) instead of
    an SVG. File clips are excluded from the `looksLikeCode` mono styling. The
    detail modal shows a "File clip" title and lists one URI per line.
  - **styles.css**: `.clip-icon .clip-file-ext` framed badge styling per the
    `ClipFileIcon` design.
  - The Plan 01 **Files** filter chip is now live.
- ✅ **05 — Clip Detail metadata** (2026-06-30): added a timestamp + source-app
  metadata strip to the Clip Detail modal.
  - **store.rs**: new `source_app: Option<String>` field on `ClipItem` with
    `#[serde(default)]` so old `history.json` files load without error.
    `add_text` / `add_image` / `add_file` each take `source_app` and store it
    (also stamped onto de-duped existing items on re-copy).
  - **clipboard.rs**: `active_window_name(backend)` captures the focused
    window title via `xdotool getactivewindow getwindowname`. X11-only; returns
    `None` on Wayland (no reliable portal-free way to read the focused app
    from a background process) or if xdotool is missing.
  - **lib.rs**: `spawn_watcher` snapshots the active window right before
    reading the clipboard and passes it through to the `add_*` calls.
  - **index.html**: new `#detail-meta` strip between the dialog head and body.
  - **main.js**: `openDetail()` renders `Copied <locale time> · From <app>`,
    falling back to `From Unknown` when `source_app` is absent (Wayland).
  - **styles.css**: `.detail-meta` flex strip with hairline divider; hidden
    via `:empty` until populated.

## Notes
- Plans 01, 04, 07 are frontend-only and safe to do first. (07 also touches
  Rust for 7f's X11 cursor positioning.)
- Plans 03, 05, 06 touch Rust; rebuild with `npm run dev` after each.
- Plan 02's launch-on-login persists the flag but does NOT register a GNOME
  autostart `.desktop` entry — that's a follow-up backend task noted in the plan.
- Plan 07's open decisions are resolved: **7e** → `radius-xl = 12px` (CSS wins);
  **7f** → X11 near-cursor, Wayland centered. See the Progress entry for details.
