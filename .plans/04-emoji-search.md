# Plan 04 — Emoji search-by-name

## Goal
The emoji picker's search box currently does nothing. `renderEmoji()` ignores
its `query` argument and always renders the full list. The design specifies
that search filters emoji by name.

## Design reference (`design.pen`)
- Screen: `Screen/EmojiPicker` (`E4nlZv`) with `EmojiSearch` (`Am9Dg`) ref
- Annotation `NoteEmoji`:
  > Emoji/Symbol Picker: Grid of frequently used symbols. Search filters by
  > name. Click to copy to clipboard. Accessible via history overlay tab.

## Files to touch
- `src/main.js` — `renderEmoji()` and the EMOJI data structure

## Implementation steps

### 1. Give each emoji a name
Replace the flat string array (src/main.js:240) with an array of
`{ ch, name }` objects. Use the canonical short names so search is useful:
```js
const EMOJI = [
  { ch: "😀", name: "grin happy face" },
  { ch: "😂", name: "joy laugh cry face" },
  { ch: "😍", name: "heart eyes love face" },
  // ... the rest of the existing 70+ chars, each with 2-4 keywords
];
```
Keep the existing character set; just add names. Keywords that help search:
emotion (happy/sad/angry), object (clipboard/pin/trash), category (arrow/
math/currency). Lowercase all names for case-insensitive matching.

### 2. Make `renderEmoji()` actually filter
```js
function renderEmoji(query) {
  const grid = $("#emoji-grid");
  grid.innerHTML = "";
  const q = (query || "").trim().toLowerCase();
  const list = q
    ? EMOJI.filter((e) => e.name.includes(q) || e.ch === q)
    : EMOJI;
  if (list.length === 0) {
    grid.innerHTML = '<div class="emoji-empty">No emoji match "' + escapeHtml(query) + '"</div>';
    return;
  }
  list.forEach((e) => {
    const cell = document.createElement("div");
    cell.className = "emoji-cell";
    cell.textContent = e.ch;
    cell.title = e.name;            // tooltip with the name
    cell.addEventListener("click", () => copyRaw(e.ch));
    grid.appendChild(cell);
  });
}
```

### 3. Wire the existing search input
The input listener at src/main.js:343 already calls `renderEmoji(searchEl.value)`
for the emoji view — so once `renderEmoji` uses the arg, it just works. No new
wiring needed.

### 4. Empty-state styling (optional, small)
```css
.emoji-empty { grid-column: 1 / -1; text-align: center; color: var(--text-muted);
  font-size: var(--text-md); padding: var(--space-xl); }
```

## Verification
1. `npm run dev` → Emoji tab.
2. Type "arrow" → only arrow emoji remain (⬆️ ⬇️ ⬅️ ➡️ ↩️ 🔄).
3. Type "heart" → ❤️ 💔 😍 remain.
4. Type gibberish "zzz" → empty-state message shows.
5. Clear the search → full grid returns.
6. `title` tooltip shows the emoji name on hover.
