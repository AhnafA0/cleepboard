# Plan 01 — Interaction States (hover / selected / focused)

## Goal

The spec (`clipboard-manager-design.md` §2, §4) requires hover, pressed,
selected, and focus states shown as **separate frames or annotated variants**.
Today these exist only as text notes. This plan adds concrete state frames for
the four most important components: **Button**, **ClipCard**, **SearchBar**,
**NavItem**.

No new reusable components are created — instead, state frames are placed in a
new **States** section inside the Design System frame so engineers can see
exactly what CSS states to implement.

## Prerequisites
- None (first plan).

## Target location
Insert a new section frame as a child of `W7zt6X` (Design System), positioned
after the existing `BKMR4` (Overlays) section. The Design System frame uses
`layout: vertical` with `gap: 32`, so a new child appended at the end will
appear below the Overlays section.

## Steps

### Step 1 — Create the States section container

`batch_design` with `filePath` = `/opt/Devin projects/cleepboard/design.pen`:

```js
({
  operations: [
    {
      op: "Insert",
      parent: "W7zt6X",
      node: {
        type: "frame",
        name: "States Section",
        layout: "vertical",
        gap: 12,
        padding: 16,
        width: "fill_container",
        fill: "#00000000",
        children: [
          { type: "text", name: "StatesLabel", content: "Interaction States",
            fill: "$text-primary", fontFamily: "Inter",
            fontSize: "$text-md", fontWeight: "600" },
          { type: "text", name: "NoteStates",
            content: "Hover = 10% darker bg / accent. Selected = accent left border 3px + bg-tertiary. Focused = accent ring 2px offset 2px. Pressed = 15% darker than hover.",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: 11, fontWeight: "normal",
            textGrowth: "fixed-width", width: 500 }
        ]
      }
    }
  ]
})
```

Read back the returned node ID — call it `STATES_SECTION_ID`.

### Step 2 — Button states row

Insert a horizontal frame into `STATES_SECTION_ID` containing four Button
state variants. Each is a standalone frame (not a ref) so the state styling is
visible.

```js
({
  operations: [
    { op: "Insert", parent: STATES_SECTION_ID, node: {
        type: "frame", name: "Button States Row", layout: "horizontal",
        gap: 12, alignItems: "center", fill: "#00000000",
        children: [
          { type: "frame", name: "Button/Primary/Hover",
            layout: "horizontal", alignItems: "center", justifyContent: "center",
            gap: 8, padding: [8, 16], cornerRadius: "$radius-md",
            fill: "#3273E6",  // ~10% darker than $accent #3B82F6
            children: [
              { type: "text", name: "t", content: "Hover",
                fill: "#FFFFFF", fontFamily: "Inter",
                fontSize: "$text-md", fontWeight: "500" } ] },
          { type: "frame", name: "Button/Primary/Pressed",
            layout: "horizontal", alignItems: "center", justifyContent: "center",
            gap: 8, padding: [8, 16], cornerRadius: "$radius-md",
            fill: "#2B66D1",  // ~15% darker than hover
            children: [
              { type: "text", name: "t", content: "Pressed",
                fill: "#FFFFFF", fontFamily: "Inter",
                fontSize: "$text-md", fontWeight: "500" } ] },
          { type: "frame", name: "Button/Primary/Focused",
            layout: "horizontal", alignItems: "center", justifyContent: "center",
            gap: 8, padding: [8, 16], cornerRadius: "$radius-md",
            fill: "$accent",
            stroke: "#FFFFFF", strokeWidth: 2, strokeAlignment: "inner",
            effect: { type: "shadow", shadowType: "outer",
                      offset: { x: 0, y: 0 }, blur: 0, spread: 2,
                      color: "$accent" },
            children: [
              { type: "text", name: "t", content: "Focused",
                fill: "#FFFFFF", fontFamily: "Inter",
                fontSize: "$text-md", fontWeight: "500" } ] },
          { type: "frame", name: "Button/Primary/Disabled",
            layout: "horizontal", alignItems: "center", justifyContent: "center",
            gap: 8, padding: [8, 16], cornerRadius: "$radius-md",
            fill: "$accent", opacity: 0.4,
            children: [
              { type: "text", name: "t", content: "Disabled",
                fill: "#FFFFFF", fontFamily: "Inter",
                fontSize: "$text-md", fontWeight: "500" } ] }
        ]
    } }
  ]
})
```

### Step 3 — ClipCard states row

Three ClipCard variants side-by-side: **hover**, **selected**, **focused**.
Each mirrors the ClipCard structure (`y4kMU`) but with state-specific fills /
strokes.

```js
({
  operations: [
    { op: "Insert", parent: STATES_SECTION_ID, node: {
        type: "frame", name: "ClipCard States Row", layout: "horizontal",
        gap: 12, alignItems: "start", fill: "#00000000",
        children: [
          // Hover
          { type: "frame", name: "ClipCard/Hover",
            layout: "horizontal", gap: 12, padding: 12,
            cornerRadius: "$radius-md", fill: "$bg-secondary",
            stroke: "$border", strokeWidth: 1, width: 200,
            children: [
              { type: "text", name: "icon", content: "T",
                fill: "$accent", fontFamily: "Inter",
                fontSize: 18, fontWeight: "700",
                width: 32, height: 32, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" },
              { type: "frame", name: "body", layout: "vertical", gap: 4,
                fill: "#00000000", children: [
                  { type: "text", name: "txt", content: "Hovered clip row...",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-md", textGrowth: "fixed-width", width: 130 },
                  { type: "text", name: "meta", content: "1 min ago",
                    fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-sm" }
                ] }
            ] },
          // Selected
          { type: "frame", name: "ClipCard/Selected",
            layout: "horizontal", gap: 12, padding: 12,
            cornerRadius: "$radius-md", fill: "$bg-tertiary",
            stroke: "$accent", strokeWidth: 2, width: 200,
            children: [
              { type: "text", name: "icon", content: "T",
                fill: "$accent", fontFamily: "Inter",
                fontSize: 18, fontWeight: "700",
                width: 32, height: 32, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" },
              { type: "frame", name: "body", layout: "vertical", gap: 4,
                fill: "#00000000", children: [
                  { type: "text", name: "txt", content: "Selected clip row...",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-md", textGrowth: "fixed-width", width: 130 },
                  { type: "text", name: "meta", content: "2 min ago",
                    fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-sm" }
                ] }
            ] },
          // Focused (keyboard)
          { type: "frame", name: "ClipCard/Focused",
            layout: "horizontal", gap: 12, padding: 12,
            cornerRadius: "$radius-md", fill: "$surface",
            stroke: "$accent", strokeWidth: 2, width: 200,
            effect: { type: "shadow", shadowType: "outer",
                      offset: { x: 0, y: 0 }, blur: 0, spread: 2,
                      color: "$accent" },
            children: [
              { type: "text", name: "icon", content: "T",
                fill: "$accent", fontFamily: "Inter",
                fontSize: 18, fontWeight: "700",
                width: 32, height: 32, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" },
              { type: "frame", name: "body", layout: "vertical", gap: 4,
                fill: "#00000000", children: [
                  { type: "text", name: "txt", content: "Keyboard-focused...",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-md", textGrowth: "fixed-width", width: 130 },
                  { type: "text", name: "meta", content: "3 min ago",
                    fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-sm" }
                ] }
            ] }
        ]
    } }
  ]
})
```

### Step 4 — SearchBar states row

Two variants: **focused** (accent border) and **with-clear-button** (shows an
× IconButton ref at the right).

```js
({
  operations: [
    { op: "Insert", parent: STATES_SECTION_ID, node: {
        type: "frame", name: "SearchBar States Row", layout: "horizontal",
        gap: 12, alignItems: "center", fill: "#00000000",
        children: [
          // Focused
          { type: "frame", name: "SearchBar/Focused",
            layout: "horizontal", alignItems: "center", gap: 8,
            padding: [0, 12], cornerRadius: "$radius-lg",
            fill: "$surface", stroke: "$accent", strokeWidth: 2,
            height: 36, width: 260,
            children: [
              { type: "text", name: "icon", content: "search",
                fill: "$accent", fontFamily: "Inter", fontSize: "$text-md" },
              { type: "text", name: "q", content: "meeti|",
                fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" }
            ] },
          // With clear button
          { type: "frame", name: "SearchBar/WithClear",
            layout: "horizontal", alignItems: "center", gap: 8,
            padding: [0, 12], cornerRadius: "$radius-lg",
            fill: "$surface", stroke: "$border", strokeWidth: 1,
            height: 36, width: 260,
            justifyContent: "space_between",
            children: [
              { type: "frame", name: "left", layout: "horizontal",
                alignItems: "center", gap: 8, fill: "#00000000",
                children: [
                  { type: "text", name: "icon", content: "search",
                    fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-md" },
                  { type: "text", name: "q", content: "meeting notes",
                    fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" }
                ] },
              { type: "text", name: "clear", content: "x",
                fill: "$text-muted", fontFamily: "Inter",
                fontSize: 16, fontWeight: "600",
                width: 20, height: 20, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" }
            ] }
        ]
    } }
  ]
})
```

### Step 5 — NavItem states row

Three variants: **default** (existing), **hover** (bg-tertiary), **active**
(accent left border + accent text).

```js
({
  operations: [
    { op: "Insert", parent: STATES_SECTION_ID, node: {
        type: "frame", name: "NavItem States Row", layout: "horizontal",
        gap: 12, alignItems: "center", fill: "#00000000",
        children: [
          { type: "frame", name: "NavItem/Hover",
            layout: "horizontal", alignItems: "center", gap: 8,
            padding: [0, 12], cornerRadius: "$radius-md",
            fill: "$bg-tertiary", height: 36, width: 140,
            children: [
              { type: "text", name: "icon", content: "settings",
                fill: "$text-primary", fontFamily: "Inter", fontSize: 16 },
              { type: "text", name: "label", content: "General",
                fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" }
            ] },
          { type: "frame", name: "NavItem/Active",
            layout: "horizontal", alignItems: "center", gap: 8,
            padding: [0, 12], cornerRadius: "$radius-md",
            fill: "$bg-tertiary", height: 36, width: 140,
            stroke: "$accent", strokeWidth: { left: 3, top: 0, right: 0, bottom: 0 },
            strokeAlignment: "inner",
            children: [
              { type: "text", name: "icon", content: "settings",
                fill: "$accent", fontFamily: "Inter", fontSize: 16 },
              { type: "text", name: "label", content: "General",
                fill: "$accent", fontFamily: "Inter",
                fontSize: "$text-md", fontWeight: "600" }
            ] }
        ]
    } }
  ]
})
```

## Verification
1. `batch_get` on `STATES_SECTION_ID` with `readDepth: 2` — confirm 5 children
   (label, note, 4 state rows).
2. `get_screenshot` on `W7zt6X` (Design System) — confirm the States section
   appears at the bottom with all state frames visible and not overflowing.
3. Check that no frame has `alignItems: "stretch"` or `"baseline"` (invalid
   values per schema).

## Commit
```
git add design.pen .plans/
git commit -m "design: add interaction state frames for Button, ClipCard, SearchBar, NavItem"
```
