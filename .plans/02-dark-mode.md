# Plan 02 — Dark Mode Mockups

## Goal

The spec (§1, §2) requires light **and dark** theme variants. Dark color
variables (`$accent-dark`, `$bg-primary-dark`, etc.) already exist but **zero
components or screens use them**. This plan creates dark-theme duplicates of the
two most important screens — **HistoryOverlay** and **SettingsPanel** — so
engineers can see the dark palette in context.

## Prerequisites
- **Plan 01** must be completed (the States section should exist, though it's
  not strictly required for this plan).

## Approach

Rather than restructuring variables into theme-axis form (risky, unclear
schema support), this plan creates **duplicate screen frames** that reference
the `*-dark` tokens directly. Each dark screen is placed in the Screens frame
(`ZoBik`) inside a new category frame **"Category: Dark Mode"**.

The dark screens are **not** built from refs (the existing components reference
light tokens). Instead they are standalone frames that mirror the light
screen structure but swap every token. This is intentional — it lets engineers
extract the exact dark-mode CSS values without resolving theme axes.

## Token mapping (light → dark)

| Light token | Dark token | Value |
|-------------|------------|-------|
| `$accent` | `$accent-dark` | `#60A5FA` |
| `$bg-primary` | `$bg-primary-dark` | `#1A1D21` |
| `$bg-secondary` | `$bg-secondary-dark` | `#252A2E` |
| `$bg-tertiary` | `$bg-tertiary-dark` | `#2F353A` |
| `$border` | `$border-dark` | `#3A4047` |
| `$surface` | `$surface-dark` | `#1E2127` |
| `$text-primary` | `$text-primary-dark` | `#F1F3F5` |
| `$text-secondary` | `$text-secondary-dark` | `#C1C7CD` |
| `$text-muted` | `$text-muted-dark` | `#868E96` |

Note: `$danger` and `$success` have no `-dark` variant — use them as-is in dark
mode (they have sufficient contrast on dark backgrounds).

## Steps

### Step 1 — Create the Dark Mode category frame

Insert into `ZoBik` (Screens). Since `ZoBik` uses `layout: vertical`, the new
category appears below the existing Configuration category.

```js
({
  operations: [
    { op: "Insert", parent: "ZoBik", node: {
        type: "frame", name: "Category: Dark Mode",
        layout: "vertical", gap: 16, padding: 24,
        cornerRadius: "$radius-lg", fill: "$bg-secondary-dark",
        children: [
          { type: "text", name: "DarkModeLabel", content: "Dark Mode",
            fill: "$text-primary-dark", fontFamily: "Inter",
            fontSize: "$text-lg", fontWeight: "700" },
          { type: "frame", name: "DarkMode Content", gap: 24,
            fill: "#00000000", children: [] }
        ]
    } }
  ]
})
```

Read back the ID of the "DarkMode Content" child frame — call it
`DARK_CONTENT_ID`.

### Step 2 — Dark HistoryOverlay

Insert a dark duplicate of `Screen/HistoryOverlay` (light version is `Qj3Ub`,
width 360). Mirror its structure: search bar, pin chip, section header, 3 clip
cards, shortcut badge.

```js
({
  operations: [
    { op: "Insert", parent: DARK_CONTENT_ID, node: {
        type: "frame", name: "Screen/HistoryOverlay/Dark",
        layout: "vertical", gap: 12, padding: 16, width: 360,
        cornerRadius: "$radius-xl", fill: "$surface-dark",
        stroke: "$border-dark", strokeWidth: 1,
        effect: { type: "shadow", shadowType: "outer",
                  offset: { x: 0, y: 8 }, blur: 32, color: "#00000050" },
        children: [
          // Search bar
          { type: "frame", name: "DarkSearch", layout: "horizontal",
            alignItems: "center", gap: 8, padding: [0, 12],
            cornerRadius: "$radius-lg", fill: "$bg-secondary-dark",
            stroke: "$border-dark", strokeWidth: 1, height: 36, width: 280,
            children: [
              { type: "text", name: "icon", content: "search",
                fill: "$text-muted-dark", fontFamily: "Inter", fontSize: "$text-md" },
              { type: "text", name: "ph", content: "Search clips...",
                fill: "$text-muted-dark", fontFamily: "Inter", fontSize: "$text-md" }
            ] },
          // Pin chip
          { type: "frame", name: "DarkPinChip", layout: "horizontal",
            alignItems: "center", gap: 4, padding: [4, 8],
            cornerRadius: 12, fill: "$bg-tertiary-dark",
            children: [
              { type: "text", name: "icon", content: "pin",
                fill: "$text-muted-dark", fontFamily: "Inter", fontSize: 10 },
              { type: "text", name: "label", content: "Pinned",
                fill: "$text-secondary-dark", fontFamily: "Inter",
                fontSize: "$text-sm", fontWeight: "500" }
            ] },
          // Section header
          { type: "frame", name: "DarkSection", layout: "horizontal",
            alignItems: "center", gap: 8, padding: [8, 0], width: 320,
            fill: "#00000000",
            children: [
              { type: "rectangle", name: "lineL", fill: "$border-dark",
                height: 1, width: 24 },
              { type: "text", name: "label", content: "Today",
                fill: "$text-muted-dark", fontFamily: "Inter",
                fontSize: "$text-sm", fontWeight: "600" },
              { type: "rectangle", name: "lineR", fill: "$border-dark",
                height: 1, width: "fill_container" }
            ] },
          // Clip card 1
          { type: "frame", name: "DarkClip1", layout: "horizontal",
            gap: 12, padding: 12, cornerRadius: "$radius-md",
            fill: "$bg-secondary-dark", stroke: "$border-dark", strokeWidth: 1,
            width: 320,
            children: [
              { type: "text", name: "icon", content: "T",
                fill: "$accent-dark", fontFamily: "Inter",
                fontSize: 18, fontWeight: "700",
                width: 32, height: 32, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" },
              { type: "frame", name: "body", layout: "vertical", gap: 4,
                fill: "#00000000", children: [
                  { type: "text", name: "txt", content: "Clipboard text content goes here...",
                    fill: "$text-primary-dark", fontFamily: "Inter",
                    fontSize: "$text-md", textGrowth: "fixed-width", width: 220 },
                  { type: "text", name: "meta", content: "2 min ago",
                    fill: "$text-muted-dark", fontFamily: "Inter", fontSize: "$text-sm" }
                ] }
            ] },
          // Clip card 2 (focused state in dark)
          { type: "frame", name: "DarkClip2Focused", layout: "horizontal",
            gap: 12, padding: 12, cornerRadius: "$radius-md",
            fill: "$bg-tertiary-dark", stroke: "$accent-dark", strokeWidth: 2,
            width: 320,
            effect: { type: "shadow", shadowType: "outer",
                      offset: { x: 0, y: 0 }, blur: 0, spread: 2,
                      color: "$accent-dark" },
            children: [
              { type: "text", name: "icon", content: "T",
                fill: "$accent-dark", fontFamily: "Inter",
                fontSize: 18, fontWeight: "700",
                width: 32, height: 32, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" },
              { type: "frame", name: "body", layout: "vertical", gap: 4,
                fill: "#00000000", children: [
                  { type: "text", name: "txt", content: "Keyboard-focused dark row...",
                    fill: "$text-primary-dark", fontFamily: "Inter",
                    fontSize: "$text-md", textGrowth: "fixed-width", width: 220 },
                  { type: "text", name: "meta", content: "5 min ago",
                    fill: "$text-muted-dark", fontFamily: "Inter", fontSize: "$text-sm" }
                ] }
            ] },
          // Clip card 3
          { type: "frame", name: "DarkClip3", layout: "horizontal",
            gap: 12, padding: 12, cornerRadius: "$radius-md",
            fill: "$bg-secondary-dark", stroke: "$border-dark", strokeWidth: 1,
            width: 320,
            children: [
              { type: "text", name: "icon", content: "T",
                fill: "$accent-dark", fontFamily: "Inter",
                fontSize: 18, fontWeight: "700",
                width: 32, height: 32, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" },
              { type: "frame", name: "body", layout: "vertical", gap: 4,
                fill: "#00000000", children: [
                  { type: "text", name: "txt", content: "Another copied snippet...",
                    fill: "$text-primary-dark", fontFamily: "Inter",
                    fontSize: "$text-md", textGrowth: "fixed-width", width: 220 },
                  { type: "text", name: "meta", content: "10 min ago",
                    fill: "$text-muted-dark", fontFamily: "Inter", fontSize: "$text-sm" }
                ] }
            ] },
          // Shortcut badge
          { type: "frame", name: "DarkBadge", layout: "horizontal",
            alignItems: "center", gap: 4, padding: [4, 8],
            cornerRadius: "$radius-sm", fill: "$bg-tertiary-dark",
            children: [
              { type: "text", name: "txt", content: "Ctrl+Shift+V",
                fill: "$text-secondary-dark", fontFamily: "Inter",
                fontSize: "$text-sm", fontWeight: "500" }
            ] }
        ]
    } },
    // Note under the dark overlay
    { op: "Insert", parent: DARK_CONTENT_ID, node: {
        type: "text", name: "NoteDarkOverlay",
        content: "Dark HistoryOverlay: uses -dark token variants. Focused row shows accent-dark ring + bg-tertiary-dark fill. Shadow alpha increased to 0x50 for dark bg.",
        fill: "$text-secondary-dark", fontFamily: "Inter",
        fontSize: 11, textGrowth: "fixed-width", width: 320
    } }
  ]
})
```

### Step 3 — Dark SettingsPanel

Insert a dark duplicate of `Screen/SettingsPanel` (light version is `sfvQi`,
width 560, height 400). Mirror sidebar + content structure.

```js
({
  operations: [
    { op: "Insert", parent: DARK_CONTENT_ID, node: {
        type: "frame", name: "Screen/SettingsPanel/Dark",
        layout: "horizontal", width: 560, height: 400,
        cornerRadius: "$radius-xl", fill: "$surface-dark",
        stroke: "$border-dark", strokeWidth: 1,
        children: [
          // Sidebar
          { type: "frame", name: "DarkSettingsSidebar",
            layout: "vertical", gap: 4, padding: 16, width: 180,
            height: "fill_container", cornerRadius: [16, 0, 0, 16],
            fill: "$bg-secondary-dark",
            children: [
              { type: "frame", name: "nav1", layout: "horizontal",
                alignItems: "center", gap: 8, padding: [0, 12],
                cornerRadius: "$radius-md", fill: "$bg-tertiary-dark",
                height: 36, width: 240,
                stroke: "$accent-dark", strokeWidth: { left: 3, top: 0, right: 0, bottom: 0 },
                strokeAlignment: "inner",
                children: [
                  { type: "text", name: "icon", content: "settings",
                    fill: "$accent-dark", fontFamily: "Inter", fontSize: 16 },
                  { type: "text", name: "label", content: "General",
                    fill: "$accent-dark", fontFamily: "Inter",
                    fontSize: "$text-md", fontWeight: "600" }
                ] },
              { type: "frame", name: "nav2", layout: "horizontal",
                alignItems: "center", gap: 8, padding: [0, 12],
                cornerRadius: "$radius-md", fill: "#00000000",
                height: 36, width: 240,
                children: [
                  { type: "text", name: "icon", content: "keyboard",
                    fill: "$text-secondary-dark", fontFamily: "Inter", fontSize: 16 },
                  { type: "text", name: "label", content: "Hotkeys",
                    fill: "$text-secondary-dark", fontFamily: "Inter", fontSize: "$text-md" }
                ] },
              { type: "frame", name: "nav3", layout: "horizontal",
                alignItems: "center", gap: 8, padding: [0, 12],
                cornerRadius: "$radius-md", fill: "#00000000",
                height: 36, width: 240,
                children: [
                  { type: "text", name: "icon", content: "database",
                    fill: "$text-secondary-dark", fontFamily: "Inter", fontSize: 16 },
                  { type: "text", name: "label", content: "Storage",
                    fill: "$text-secondary-dark", fontFamily: "Inter", fontSize: "$text-md" }
                ] },
              { type: "frame", name: "nav4", layout: "horizontal",
                alignItems: "center", gap: 8, padding: [0, 12],
                cornerRadius: "$radius-md", fill: "#00000000",
                height: 36, width: 240,
                children: [
                  { type: "text", name: "icon", content: "palette",
                    fill: "$text-secondary-dark", fontFamily: "Inter", fontSize: 16 },
                  { type: "text", name: "label", content: "Appearance",
                    fill: "$text-secondary-dark", fontFamily: "Inter", fontSize: "$text-md" }
                ] }
            ] },
          // Content
          { type: "frame", name: "DarkSettingsContent",
            layout: "vertical", gap: 16, padding: 24, width: "fill_container",
            fill: "#00000000",
            children: [
              { type: "text", name: "title", content: "General",
                fill: "$text-primary-dark", fontFamily: "Inter",
                fontSize: "$text-xl", fontWeight: "700" },
              // Toggle row 1
              { type: "frame", name: "row1", layout: "horizontal",
                alignItems: "center", gap: 8, width: "fill_container",
                justifyContent: "space_between", fill: "#00000000",
                children: [
                  { type: "text", name: "label", content: "Auto-paste on select",
                    fill: "$text-primary-dark", fontFamily: "Inter", fontSize: "$text-md" },
                  // Dark toggle (on)
                  { type: "frame", name: "toggle", layout: "none", width: 36, height: 20,
                    cornerRadius: 10, fill: "$accent-dark",
                    children: [
                      { type: "ellipse", name: "thumb", fill: "#FFFFFF",
                        width: 16, height: 16, x: 18, y: 2 }
                    ] }
                ] },
              // Toggle row 2
              { type: "frame", name: "row2", layout: "horizontal",
                alignItems: "center", gap: 8, width: "fill_container",
                justifyContent: "space_between", fill: "#00000000",
                children: [
                  { type: "text", name: "label", content: "Run at startup",
                    fill: "$text-primary-dark", fontFamily: "Inter", fontSize: "$text-md" },
                  // Dark toggle (off)
                  { type: "frame", name: "toggle", layout: "none", width: 36, height: 20,
                    cornerRadius: 10, fill: "$bg-tertiary-dark",
                    children: [
                      { type: "ellipse", name: "thumb", fill: "$text-muted-dark",
                        width: 16, height: 16, x: 2, y: 2 }
                    ] }
                ] },
              // Shortcut label + badge
              { type: "frame", name: "shortcutRow", layout: "horizontal",
                alignItems: "center", gap: 8, fill: "#00000000",
                children: [
                  { type: "text", name: "label", content: "Open history: ",
                    fill: "$text-secondary-dark", fontFamily: "Inter", fontSize: "$text-md" },
                  { type: "frame", name: "badge", layout: "horizontal",
                    alignItems: "center", gap: 4, padding: [4, 8],
                    cornerRadius: "$radius-sm", fill: "$bg-tertiary-dark",
                    children: [
                      { type: "text", name: "txt", content: "Ctrl+Shift+V",
                        fill: "$text-secondary-dark", fontFamily: "Inter",
                        fontSize: "$text-sm", fontWeight: "500" }
                    ] }
                ] }
            ] }
        ]
    } },
    // Note
    { op: "Insert", parent: DARK_CONTENT_ID, node: {
        type: "text", name: "NoteDarkSettings",
        content: "Dark SettingsPanel: sidebar uses bg-secondary-dark, active nav has accent-dark left border. Toggles: on=accent-dark, off=bg-tertiary-dark with muted thumb.",
        fill: "$text-secondary-dark", fontFamily: "Inter",
        fontSize: 11, textGrowth: "fixed-width", width: 481
    } }
  ]
})
```

## Verification
1. `batch_get` on the "Category: Dark Mode" frame with `readDepth: 2` —
   confirm 3 children (label, content frame, 2 screens + 2 notes inside
   content).
2. `get_screenshot` on the dark HistoryOverlay and dark SettingsPanel node IDs
   — confirm:
   - Backgrounds are dark (`#1E2127` surface, `#252A2E` sidebar).
   - Text is light and readable.
   - The focused clip card has a visible accent-blue ring.
   - No layout overflow (clip cards fit within 360px width).
3. If any dark screen looks wrong, read the node at `readDepth: 3` and fix
   individual properties with `update_properties`.

## Commit
```
git add design.pen .plans/
git commit -m "design: add dark mode mockups for HistoryOverlay and SettingsPanel"
```
