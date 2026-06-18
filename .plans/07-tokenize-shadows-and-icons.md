# Plan 07 — Tokenize Shadows & Add Icon System

## Goal

Two final cleanup tasks to bring the design to full handoff quality:

1. **Tokenize shadows** — today every screen and component hardcodes shadow
   values inline (e.g., `{ blur: 32, color: "#00000030", offset: { x: 0, y: 8 } }`).
   This plan adds `$shadow-*` variables and replaces all inline shadows with
   variable references, so shadow elevation is centralized.
2. **Add an icon system** — today all icons are literal text glyphs (`"x"`,
   `"search"`, `"pin"`, `"settings"`). This plan documents an icon-font
   approach using Lucide (open-source, SF-Symbols-like, outlined, minimal —
   matching the spec's §1 iconography requirement) and creates a reusable
   `Component/Icon` wrapper.

## Prerequisites
- **Plan 05** completed (orphaned primitives extracted, audit clean).
- **Plan 06** completed (all edge states added — so all screens that need
  shadow tokenization exist).

## Part A — Tokenize Shadows

### Step 1 — Create shadow variables

Use `set_variables` to add shadow tokens. The .pen shadow effect schema is
`{ type: "shadow", shadowType, offset: {x,y}, blur, spread, color }`. Since
variables can only be single values (not compound objects), we create
**separate variables for each shadow property** following a naming convention,
then reference them in effects.

> **Important:** Check whether the .pen schema supports variable references
> inside effect objects (e.g., `"blur": "$shadow-overlay-blur"`). If it does
> not, the alternative is to create **color-only** shadow variables
> (`$shadow-color-sm`, `$shadow-color-md`, `$shadow-color-lg`) and keep
> blur/offset as literal numbers, but at least the shadow *color* (which
> varies between light/dark mode) is tokenized.

**Add these variables via `set_variables`:**

```js
// Shadow colors (light mode — subtle dark with alpha)
"$shadow-color-sm":  { type: "color", value: "#00000020" },  // small elevation
"$shadow-color-md":  { type: "color", value: "#00000030" },  // medium elevation
"$shadow-color-lg":  { type: "color", value: "#00000040" },  // large elevation (overlays)
// Shadow colors (dark mode — deeper black with more alpha)
"$shadow-color-sm-dark": { type: "color", value: "#00000040" },
"$shadow-color-md-dark": { type: "color", value: "#00000050" },
"$shadow-color-lg-dark": { type: "color", value: "#00000060" }
```

### Step 2 — Define shadow elevation tiers

Document the three tiers in the Tokens Section (`pIv7o`). Add a note:

```js
({
  operations: [
    { op: "Insert", parent: "pIv7o", node: {
        type: "text", name: "NoteShadows",
        content: "Shadow elevation tiers:\n- sm: blur 16, offset y 4, color $shadow-color-sm — tray menu, tooltips\n- md: blur 24, offset y 4, color $shadow-color-md — dialogs, popovers\n- lg: blur 32, offset y 8, color $shadow-color-lg — main overlay, screens\nDark mode: use -dark shadow color variants (higher alpha).",
        fill: "$text-secondary", fontFamily: "Inter",
        fontSize: 11, textGrowth: "fixed-width", width: 260
    } }
  ]
})
```

### Step 3 — Replace inline shadow colors with token references

Update every component and screen that has an inline shadow. The key nodes
and their current shadow colors:

| Node | Name | Current color | New color |
|------|------|---------------|-----------|
| `Qj3Ub` | Screen/HistoryOverlay | `#00000030` | `$shadow-color-lg` |
| `I0lwcw` | Screen/ClipDetail | `#00000030` | `$shadow-color-lg` |
| `E4nlZv` | Screen/EmojiPicker | `#00000030` | `$shadow-color-lg` |
| `L3WX9` | Screen/TrayMenu | `#00000020` | `$shadow-color-sm` |
| `pHjtP` | Component/DialogFrame | `#00000020` | `$shadow-color-md` |
| (Plan 02) | Screen/HistoryOverlay/Dark | `#00000050` | `$shadow-color-lg-dark` |
| (Plan 06) | Screen/HistoryOverlay/SearchEmpty | `#00000030` | `$shadow-color-lg` |
| (Plan 06) | Screen/ClearHistoryConfirm | `#00000020` | `$shadow-color-md` |

For each, use `update_properties` to replace the shadow `color` field:

```js
({
  operations: [
    { op: "update_properties", id: "Qj3Ub",
      properties: {
        effect: { type: "shadow", shadowType: "outer",
                  offset: { x: 0, y: 8 }, blur: 32, spread: 0,
                  color: "$shadow-color-lg" }
      } },
    { op: "update_properties", id: "I0lwcw",
      properties: {
        effect: { type: "shadow", shadowType: "outer",
                  offset: { x: 0, y: 8 }, blur: 32, spread: 0,
                  color: "$shadow-color-lg" }
      } },
    { op: "update_properties", id: "E4nlZv",
      properties: {
        effect: { type: "shadow", shadowType: "outer",
                  offset: { x: 0, y: 8 }, blur: 32, spread: 0,
                  color: "$shadow-color-lg" }
      } },
    { op: "update_properties", id: "L3WX9",
      properties: {
        effect: { type: "shadow", shadowType: "outer",
                  offset: { x: 0, y: 4 }, blur: 16, spread: 0,
                  color: "$shadow-color-sm" }
      } },
    { op: "update_properties", id: "pHjtP",
      properties: {
        effect: { type: "shadow", shadowType: "outer",
                  offset: { x: 0, y: 4 }, blur: 24, spread: 0,
                  color: "$shadow-color-md" }
      } }
  ]
})
```

> For nodes created in Plans 02 and 06, read their IDs first with `batch_get`
> on the parent wrapper frames, then include them in the update operations
> above. The dark overlay should use `$shadow-color-lg-dark`.

### Step 4 — Verify shadow tokenization

Run `batch_get` on all screen nodes with `resolveVariables: true` and confirm
every `effect.color` resolves to a `#000000xx` value derived from a
`$shadow-color-*` variable (not a hardcoded hex).

## Part B — Icon System

### Step 5 — Create Component/Icon

A reusable icon wrapper that standardizes size, color, and font. Insert into
the Inputs Section (`i25hSE`).

The approach: use **Lucide** icon font (or the `lucide` npm package which
provides SVG icons). In the .pen file, we represent icons as text nodes using
a consistent component wrapper. The `content` field holds the icon name (e.g.,
`"search"`, `"x"`, `"pin"`, `"settings"`, `"clipboard"`, `"keyboard"`,
`"database"`, `"palette"`). Engineers will replace these with actual Lucide
SVGs at implementation time.

```js
({
  operations: [
    { op: "Insert", parent: "i25hSE", node: {
        type: "frame", name: "Component/Icon",
        reusable: true,
        layout: "none", width: 20, height: 20,
        fill: "#00000000",
        children: [
          { type: "text", name: "IconGlyph", content: "circle",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: 20, textAlign: "center", textAlignVertical: "middle",
            textGrowth: "fixed-width-height",
            width: 20, height: 20, x: 0, y: 0 }
        ]
    } }
  ]
})
```

Read back the ID — call it `ICON_ID`.

### Step 6 — Document the icon system

Add a note in the Tokens Section:

```js
({
  operations: [
    { op: "Insert", parent: "pIv7o", node: {
        type: "text", name: "NoteIcons",
        content: "Icon system: Lucide (lucide.dev) — open-source, outlined, minimal. Sizes: 16 (inline), 20 (default), 32 (empty states). Color follows context: $text-secondary (default), $text-muted (disabled), $accent (active/selected), $danger (destructive). Component/Icon wraps a text glyph placeholder; engineers replace with Lucide SVG at build time.",
        fill: "$text-secondary", fontFamily: "Inter",
        fontSize: 11, textGrowth: "fixed-width", width: 260
    } }
  ]
})
```

### Step 7 — Create an icon gallery

Add a row of Icon refs showing the key icons used in the app, so engineers can
see the full set. Insert into the Inputs Section.

```js
({
  operations: [
    { op: "Insert", parent: "i25hSE", node: {
        type: "frame", name: "Icon Gallery", layout: "horizontal",
        gap: 12, alignItems: "center", fill: "#00000000",
        children: [
          { type: "ref", ref: ICON_ID, name: "IconSearch",
            overrides: { "IconGlyph/content": "search" } },
          { type: "ref", ref: ICON_ID, name: "IconX",
            overrides: { "IconGlyph/content": "x" } },
          { type: "ref", ref: ICON_ID, name: "IconPin",
            overrides: { "IconGlyph/content": "pin" } },
          { type: "ref", ref: ICON_ID, name: "IconSettings",
            overrides: { "IconGlyph/content": "settings" } },
          { type: "ref", ref: ICON_ID, name: "IconClipboard",
            overrides: { "IconGlyph/content": "clipboard" } },
          { type: "ref", ref: ICON_ID, name: "IconKeyboard",
            overrides: { "IconGlyph/content": "keyboard" } },
          { type: "ref", ref: ICON_ID, name: "IconDatabase",
            overrides: { "IconGlyph/content": "database" } },
          { type: "ref", ref: ICON_ID, name: "IconPalette",
            overrides: { "IconGlyph/content": "palette" } },
          { type: "ref", ref: ICON_ID, name: "IconCopy",
            overrides: { "IconGlyph/content": "copy" } },
          { type: "ref", ref: ICON_ID, name: "IconTrash",
            overrides: { "IconGlyph/content": "trash" } }
        ]
    } }
  ]
})
```

> If ref overrides for text content are not supported, create standalone
> Icon frames for each icon in the gallery. The important deliverable is the
> visual reference set + the documentation note.

### Step 8 — Final checklist update

Update the Behavior Spec checklist (`Fr2qc`) to mark all items complete:

```js
({
  operations: [
    { op: "update_properties", id: "Fr2qc",
      properties: {
        content: "[x] Brand tokens finalized\n[x] Design system components created and reusable\n[x] All screens mocked using component refs\n[x] Component audit passed - no orphaned primitives\n[x] Interaction states documented (hover/selected/focused)\n[x] Dark mode mockups added\n[x] Image/file clip variants added\n[x] Settings sub-panels (Hotkeys/Storage/Appearance) added\n[x] Edge states added (search-empty, clear-confirm, permission-denied)\n[x] Shadows tokenized\n[x] Icon system documented (Lucide)\n[ ] Behavior spec agreed upon\n[ ] Tech stack chosen\n[ ] Feature backlog prioritized"
      }
    }
  ]
})
```

The last 3 items (behavior spec agreed, tech stack chosen, feature backlog)
are **business decisions**, not design tasks — they remain unchecked until the
team signs off.

## Verification
1. `get_variables` — confirm 6 new `$shadow-color-*` variables exist.
2. `batch_get` on `Qj3Ub`, `I0lwcw`, `E4nlZv`, `L3WX9`, `pHjtP` with
   `resolveVariables: true` — confirm each `effect.color` resolves from a
   `$shadow-color-*` variable.
3. `batch_get` on `i25hSE` — confirm `Component/Icon` exists (reusable) and
   the Icon Gallery row is present.
4. `batch_get` on `pIv7o` — confirm shadow and icon documentation notes exist.
5. `get_screenshot` on `W7zt6X` (Design System) — confirm the Icon Gallery
   appears in the Inputs section and the shadow/icon notes are in the Tokens
   section.
6. `get_screenshot` on `Qj3Ub` — confirm the overlay shadow still looks
   correct (tokenization shouldn't change visual appearance, only the source
   of the color value).

## Commit
```
git add design.pen .plans/
git commit -m "design: tokenize shadows, add Lucide icon system, finalize handoff checklist"
```

## Post-Plan — Design Handoff Complete

After Plan 07 is committed, the `design.pen` file is ready for engineering
handoff. The remaining unchecked items in the checklist are business decisions:
- Behavior spec sign-off
- Tech stack confirmation (Tauri recommended in the spec)
- Feature backlog prioritization

Once those are agreed, the coding phase can begin using the Tauri + Rust +
Web frontend stack recommended in the Behavior Spec frame.
