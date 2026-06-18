# Plan 03 — Clip Variants & Keyboard-Focus Row

## Goal

The spec (§2) requires ClipCard variants for **text, image, and file** clips.
Today only the text variant exists (`y4kMU`). This plan:
1. Adds **image-clip** and **file-clip** ClipCard variants as new reusable
   components in the Design System.
2. Adds a **keyboard-focused row** to the HistoryOverlay screen to show the
   arrow-key navigation state.
3. Adds a **filter-chip row** (All / Text / Images / Files / Links) at the top
   of the HistoryOverlay — a core feature for a clipboard manager.

## Prerequisites
- **Plan 01** completed (States section exists, so the focused-state visual
  language is established).

## Steps

### Step 1 — Create reusable ClipCard/Image component

Insert into the Cards Section (`EBkil`, child of `ymokX` Content frame), after
the existing `y4kMU` text ClipCard.

```js
({
  operations: [
    { op: "Insert", parent: "EBkil", node: {
        type: "frame", name: "Component/ClipCard/Image",
        reusable: true,
        layout: "horizontal", gap: 12, padding: 12, width: 320,
        cornerRadius: "$radius-md", fill: "$surface",
        stroke: "$border", strokeWidth: 1,
        children: [
          // Thumbnail placeholder (image)
          { type: "rectangle", name: "ClipThumb",
            fill: "$bg-tertiary", cornerRadius: "$radius-sm",
            width: 48, height: 48 },
          { type: "frame", name: "ClipContentFrame", layout: "vertical",
            gap: 4, fill: "#00000000",
            children: [
              { type: "text", name: "ClipText", content: "Screenshot_2024-06-19.png",
                fill: "$text-primary", fontFamily: "Inter",
                fontSize: "$text-md", textGrowth: "fixed-width", width: 220 },
              { type: "text", name: "ClipMeta", content: "Image - 1280x720 - 5 min ago",
                fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-sm" }
            ] }
        ]
    } }
  ]
})
```

### Step 2 — Create reusable ClipCard/File component

```js
({
  operations: [
    { op: "Insert", parent: "EBkil", node: {
        type: "frame", name: "Component/ClipCard/File",
        reusable: true,
        layout: "horizontal", gap: 12, padding: 12, width: 320,
        cornerRadius: "$radius-md", fill: "$surface",
        stroke: "$border", strokeWidth: 1,
        children: [
          // File icon placeholder
          { type: "frame", name: "ClipFileIcon",
            layout: "vertical", alignItems: "center", justifyContent: "center",
            width: 48, height: 48, cornerRadius: "$radius-sm",
            fill: "$bg-tertiary",
            children: [
              { type: "text", name: "ext", content: "PDF",
                fill: "$danger", fontFamily: "Inter",
                fontSize: "$text-sm", fontWeight: "700" }
            ] },
          { type: "frame", name: "ClipContentFrame", layout: "vertical",
            gap: 4, fill: "#00000000",
            children: [
              { type: "text", name: "ClipText", content: "report-Q2-final.pdf",
                fill: "$text-primary", fontFamily: "Inter",
                fontSize: "$text-md", textGrowth: "fixed-width", width: 220 },
              { type: "text", name: "ClipMeta", content: "File - 2.4 MB - 12 min ago",
                fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-sm" }
            ] }
        ]
    } }
  ]
})
```

### Step 3 — Create reusable FilterChip component

A small pill for the filter row. Insert into the Cards Section.

```js
({
  operations: [
    { op: "Insert", parent: "EBkil", node: {
        type: "frame", name: "Component/FilterChip",
        reusable: true,
        layout: "horizontal", alignItems: "center", gap: 4,
        padding: [4, 10], cornerRadius: 100,
        fill: "$bg-tertiary",
        children: [
          { type: "text", name: "FilterLabel", content: "All",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: "$text-sm", fontWeight: "500" }
        ]
    } }
  ]
})
```

### Step 4 — Add filter-chip row to HistoryOverlay

Insert a horizontal row of FilterChip refs at the **top** of the HistoryOverlay
(`Qj3Ub`), before the search bar. Since `Qj3Ub` uses `layout: vertical`, we
need to insert at index 0. Use `Insert` with `index: 0`.

> **Note:** Check the schema for whether `Insert` supports an `index` field for
> ordering within a layout parent. If not, insert the filter row as a child and
> then use `reorder` if available; otherwise insert it and accept it appears
> after the badge (then read the screen and manually note the desired order for
> the engineer).

```js
({
  operations: [
    { op: "Insert", parent: "Qj3Ub", index: 0, node: {
        type: "frame", name: "HistoryFilterRow",
        layout: "horizontal", gap: 6, fill: "#00000000",
        children: [
          { type: "ref", ref: "FilterChip_ID", name: "FilterAll",
            overrides: { "FilterLabel/content": "All" } },
          { type: "ref", ref: "FilterChip_ID", name: "FilterText",
            overrides: { "FilterLabel/content": "Text" } },
          { type: "ref", ref: "FilterChip_ID", name: "FilterImages",
            overrides: { "FilterLabel/content": "Images" } },
          { type: "ref", ref: "FilterChip_ID", name: "FilterFiles",
            overrides: { "FilterLabel/content": "Files" } },
          { type: "ref", ref: "FilterChip_ID", name: "FilterLinks",
            overrides: { "FilterLabel/content": "Links" } }
        ]
    } }
  ]
})
```

> **Important:** Replace `FilterChip_ID` with the actual node ID returned from
> Step 3. If the `overrides` syntax for ref content is not supported by the
> schema, instead create 5 standalone FilterChip frames (not refs) with the
> correct label content. The first chip ("All") should use `fill: "$accent"`
> and `FilterLabel.fill: "#FFFFFF"` to show the active state.

**Fallback (if ref overrides don't work):** Create 5 standalone frames:
```js
({
  operations: [
    { op: "Insert", parent: "Qj3Ub", index: 0, node: {
        type: "frame", name: "HistoryFilterRow",
        layout: "horizontal", gap: 6, fill: "#00000000",
        children: [
          { type: "frame", name: "FilterAll", layout: "horizontal",
            alignItems: "center", gap: 4, padding: [4, 10], cornerRadius: 100,
            fill: "$accent",
            children: [ { type: "text", name: "l", content: "All",
              fill: "#FFFFFF", fontFamily: "Inter",
              fontSize: "$text-sm", fontWeight: "500" } ] },
          { type: "frame", name: "FilterText", layout: "horizontal",
            alignItems: "center", gap: 4, padding: [4, 10], cornerRadius: 100,
            fill: "$bg-tertiary",
            children: [ { type: "text", name: "l", content: "Text",
              fill: "$text-secondary", fontFamily: "Inter",
              fontSize: "$text-sm", fontWeight: "500" } ] },
          { type: "frame", name: "FilterImages", layout: "horizontal",
            alignItems: "center", gap: 4, padding: [4, 10], cornerRadius: 100,
            fill: "$bg-tertiary",
            children: [ { type: "text", name: "l", content: "Images",
              fill: "$text-secondary", fontFamily: "Inter",
              fontSize: "$text-sm", fontWeight: "500" } ] },
          { type: "frame", name: "FilterFiles", layout: "horizontal",
            alignItems: "center", gap: 4, padding: [4, 10], cornerRadius: 100,
            fill: "$bg-tertiary",
            children: [ { type: "text", name: "l", content: "Files",
              fill: "$text-secondary", fontFamily: "Inter",
              fontSize: "$text-sm", fontWeight: "500" } ] },
          { type: "frame", name: "FilterLinks", layout: "horizontal",
            alignItems: "center", gap: 4, padding: [4, 10], cornerRadius: 100,
            fill: "$bg-tertiary",
            children: [ { type: "text", name: "l", content: "Links",
              fill: "$text-secondary", fontFamily: "Inter",
              fontSize: "$text-sm", fontWeight: "500" } ] }
        ]
    } }
  ]
})
```

### Step 5 — Make HistoryClip2 the keyboard-focused row

Update the existing `p8wZs` (HistoryClip2) to show the focused state: accent
border 2px + accent ring shadow. Use `update_properties`.

```js
({
  operations: [
    { op: "update_properties", id: "p8wZs",
      properties: {
        fill: "$bg-tertiary",
        stroke: "$accent",
        strokeWidth: 2,
        effect: { type: "shadow", shadowType: "outer",
                  offset: { x: 0, y: 0 }, blur: 0, spread: 2,
                  color: "$accent" }
      }
    }
  ]
})
```

### Step 6 — Replace HistoryClip3 with an image-clip ref

To show variety in the overlay, replace the third clip card (`I839yi`) with a
ref to the new `Component/ClipCard/Image`. First read the Image ClipCard ID
from Step 1, then:

```js
({
  operations: [
    { op: "delete", id: "I839yi" },
    { op: "Insert", parent: "Qj3Ub", node: {
        type: "ref", ref: "ClipCardImage_ID", name: "HistoryClip3Image"
      }
    }
  ]
})
```

> Replace `ClipCardImage_ID` with the actual ID from Step 1.

### Step 7 — Add a note about the new overlay features

Insert a text note inside the HistoryOverlay Wrapper (`yS2BP`), after the
existing `NoteHistory`:

```js
({
  operations: [
    { op: "Insert", parent: "yS2BP", node: {
        type: "text", name: "NoteHistoryUpdates",
        content: "Filter chips: All/Text/Images/Files/Links. Active chip = accent fill. Keyboard focus: arrow keys move focus ring between rows, Enter pastes. Clip 2 shows focused state, Clip 3 shows image variant.",
        fill: "$text-secondary", fontFamily: "Inter",
        fontSize: 11, textGrowth: "fixed-width", width: 320
    } }
  ]
})
```

## Verification
1. `batch_get` on `EBkil` with `readDepth: 2` — confirm it now contains
   ClipCard (text), ClipCard/Image, ClipCard/File, FilterChip, plus existing
   PinChip/SectionHeader/ScrollableList.
2. `batch_get` on `Qj3Ub` with `readDepth: 2` — confirm children order:
   FilterRow, SearchBar, PinChip, SectionHeader, Clip1, Clip2 (focused),
   Clip3 (image ref), Badge.
3. `get_screenshot` on `Qj3Ub` — confirm:
   - Filter chips visible at top, "All" highlighted in accent blue.
   - Clip2 has a visible accent ring (focused state).
   - Clip3 shows an image thumbnail (gray square) with image filename.
   - No overflow at 360px width.
4. `get_screenshot` on the new ClipCard/Image and ClipCard/File components —
   confirm thumbnails render correctly.

## Commit
```
git add design.pen .plans/
git commit -m "design: add image/file ClipCard variants, filter chips, keyboard-focus row in overlay"
```
