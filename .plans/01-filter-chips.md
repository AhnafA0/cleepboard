# Plan 01 — Filter chips on the History Overlay

## Goal
Add the `All / Text / Images / Files / Links` filter chip row that `design.pen`
specifies for the History Overlay. This is the single most visible gap between
the mockups and the running app.

## Design reference (`design.pen`)
- Screen: `Screen/HistoryOverlay` (`Qj3Ub`) → child frame `HistoryFilterRow` (`AfFKN`)
- Component: `Component/FilterChip` (`q8YZS8`) — pill with a label
- Annotation `NoteHistoryUpdates`:
  > Filter chips: All/Text/Images/Files/Links. Active chip = accent fill.
  > Keyboard focus: arrow keys move focus ring between rows, Enter pastes.
  > Clip 2 shows focused state, Clip 3 shows image variant.

The active chip is filled with `--accent` and white text; inactive chips use
`--bg-secondary` + `--text-secondary` with a `--border` stroke.

## Files to touch
- `src/index.html` — add the chip row above `#list` inside `#view-history`
- `src/styles.css` — add `.filter-row` and `.filter-chip` rules
- `src/main.js` — chip state, wiring, and apply `activeFilter` in `applyFilter()`

## Implementation steps

### 1. HTML
Insert immediately before `<div id="list">` inside `#view-history`:
```html
<div id="filter-row" class="filter-row">
  <button class="filter-chip active" data-filter="all">All</button>
  <button class="filter-chip" data-filter="text">Text</button>
  <button class="filter-chip" data-filter="image">Images</button>
  <button class="filter-chip" data-filter="file">Files</button>
  <button class="filter-chip" data-filter="link">Links</button>
</div>
```

### 2. CSS
```css
.filter-row { display: flex; gap: var(--space-xs); flex-wrap: wrap; }
.filter-chip {
  font-family: inherit; font-size: var(--text-sm); font-weight: 600;
  padding: 4px 10px; border-radius: 999px; cursor: pointer;
  background: var(--bg-secondary); color: var(--text-secondary);
  border: 2px solid var(--border); transition: background 0.12s, color 0.12s;
}
.filter-chip:hover { background: var(--bg-tertiary); }
.filter-chip.active {
  background: var(--accent); color: #fff; border-color: var(--accent);
}
```
Add a keyboard focus ring (per the uniform-stroke rule in
`clipboard-manager-design.md`):
```css
.filter-chip:focus-visible {
  outline: none; box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 40%, transparent);
}
```

### 3. JS
- Add `let activeFilter = "all";` to the state block.
- In `applyFilter()` (src/main.js:50), after the search filter, apply the kind filter:
  ```js
  filtered = filtered.filter((i) => {
    if (activeFilter === "all") return true;
    if (activeFilter === "link")
      return i.kind === "text" && /^https?:\/\//.test((i.text || "").trim());
    if (activeFilter === "file") return i.kind === "file"; // see Plan 03
    return i.kind === activeFilter; // "text" | "image"
  });
  ```
  Keep the existing search `q` filter as the first step, then narrow by kind.
- Wire chip clicks: toggle `.active` on the clicked chip, set `activeFilter`,
  call `applyFilter()`.
- Reset `activeFilter = "all"` and clear `.active` chips in the `overlay-shown`
  listener (src/main.js:413) alongside the existing `searchEl.value = ""` reset.

## Notes / dependencies
- The **Files** chip will filter nothing until Plan 03 (file-clip variant) lands.
  That's fine — ship the chip row now; it simply shows an empty state for Files
  until file clips exist. No code change needed here.
- The **Links** chip reuses the same URL regex already in `iconFor()` (src/main.js:40).
  Consider extracting it to a `const URL_RE = /^https?:\/\//;` shared helper.

## Verification
1. `npm run dev`
2. Copy some text, an image, and a URL; confirm each chip narrows the list.
3. `All` shows everything; switching views (emoji/settings) and back preserves
   the active filter? Decide: reset to `all` on view switch OR preserve. The
   design implies reset on overlay reopen (matches `overlay-shown` reset).
4. Tab/Shift-Tab through chips — focus ring should be visible.
5. Check dark mode (`Settings → Theme → Dark`) — chips must use accent-dark.
