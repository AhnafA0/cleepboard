# Plan 07 — Interaction-state polish (bundle)

## Goal
Small mismatches between the design's interaction states and the
implementation. Each is tiny on its own; bundled here so they ship together.

## Items

### 7a. Distinct focus vs selected state for ClipCard
**Design:** `ClipCard/Selected` (uniform accent border + tinted bg) and
`ClipCard/Focused` (accent ring offset 2px) are separate frames in the States
Section. `NoteStates`: "Focused = accent ring 2px offset 2px."
**Impl:** only `.clip.selected` exists; keyboard focus and selection share it.
**Fix:** add `.clip.focused { box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 50%, transparent); }`
and apply `.focused` (in addition to `.selected`) to the keyboard-highlighted
row in `highlightSelected()` (src/main.js:161). Mouse hover sets `.selected`
only; keyboard nav sets both. Or simpler: keep one class but make the keyboard
row use the ring via a `data-nav="kbd"` attribute selector. Pick the cleaner
of the two.
**Files:** `src/styles.css`, `src/main.js`.

### 7b. Button focus ring
**Design:** `Button/Primary/Focused` (`l3jSv`) shows an accent focus ring.
**Impl:** buttons have no `:focus-visible` style.
**Fix:**
```css
.btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 40%, transparent);
}
```
Apply to `.btn`, `.icon-btn`, `.tab` for full keyboard accessibility.
**Files:** `src/styles.css`.

### 7c. Ghost button class
**Design:** `Component/Button/Ghost` (`etzbt`) is a named variant.
**Impl:** no `.btn.ghost` — ghost styling only exists implicitly on
`.icon-btn` (transparent bg).
**Fix:** add
```css
.btn.ghost { background: transparent; color: var(--text-secondary); border-color: transparent; }
.btn.ghost:hover { background: var(--bg-secondary); color: var(--text-primary); }
```
Not currently used in the UI, but add it for parity so future screens can use
it without inventing a new class.
**Files:** `src/styles.css`.

### 7d. SectionHeader divider lines
**Design:** `Component/SectionHeader` (`d7lx2E`) has `SectionLineL` + `label`
+ `SectionLineR` rectangles flanking the label.
**Impl:** `.section-header` is just label + count, no lines.
**Fix:** add flanking lines via pseudo-elements:
```css
.section-header::before, .section-header::after {
  content: ""; height: 1px; background: var(--border); flex: 1;
}
```
Keep the count where it is (after the label, before the right line) by
wrapping label+count in a non-flex span, OR drop the count and rely on the
design's lines-only look. Decide based on which reads better at 380px width.
**Files:** `src/styles.css`.

### 7e. Reconcile `radius-xl` token
**Discrepancy:** `design.pen` variable `radius-xl = 12`, but the `NoteTokens`
annotation says "radius 4-8-12-16" and the impl uses `16px`. The overlay shell
(`.overlay`) uses `--radius-xl` for its corners.
**Fix:** pick ONE value. Recommendation: set the `design.pen` variable to 16
to match the annotation and the impl (16px reads better on a 380px overlay
than 12px). Use the Pencil MCP `batch_design` to update the variable, then
verify all frames using `$radius-xl` re-render correctly. If you'd rather not
touch the design file, change `--radius-xl: 16px` → `12px` in
`src/styles.css` to match the variable. **Decide which source of truth wins**
before editing — don't leave them inconsistent.
**Files:** `design.pen` (via Pencil MCP) OR `src/styles.css`.

### 7f. Near-cursor overlay positioning (platform-limited — investigate first)
**Spec:** `NoteHistory` / `BehaviorBody`: "Appears near cursor."
**Impl:** `center: true` in `tauri.conf.json` (src-tauri/tauri.conf.json:24).
**Status:** likely **infeasible on Wayland** — getting the pointer position
from a background process requires GNOME Shell DBus introspection, and
Wayland clients can't freely position their own windows. On X11 it's trivial
(`xdotool getmouselocation` + `set_position`).
**Recommendation:** implement near-cursor on X11 only; keep `center` on
Wayland. Gate with the existing `Backend` detection. If the Wayland path is
too brittle, **close this item as won't-fix on Wayland** and add a note to
the behavior spec in `design.pen`. Do NOT ship a half-working cursor-follow
that jumps to the wrong monitor.
**Files:** `src-tauri/src/lib.rs` (`show_overlay`), `src-tauri/tauri.conf.json`.

## Verification
- 7a: Tab/arrow through the list — focused row shows a ring distinct from the
  selected fill. Mouse-click a row — only the fill, no ring.
- 7b: Tab to the "Register" button — accent ring appears.
- 7c: (no runtime test until a ghost button is used; just CSS correctness.)
- 7d: Section headers show flanking hairlines in both light and dark mode.
- 7e: overlay corner radius matches whichever source of truth you picked,
  and `design.pen` + `styles.css` agree.
- 7f: X11 — overlay opens near the pointer. Wayland — overlay opens centered
  (no regression).
