# Plan 06 — Edge States (search-empty, clear-confirm, permission-denied)

## Goal

A production clipboard manager needs edge-state mockups that engineers can
implement against. This plan adds three:

1. **Search-no-results** — shown inside the HistoryOverlay when a filter/search
   yields no clips.
2. **Clear-history confirmation dialog** — uses the existing `DialogFrame`
   component (`pHjtP`) which is currently never instantiated in any screen.
3. **Clipboard-permission-denied** — macOS requires explicit clipboard
   permission; this screen shows the denied state with a "Open System Settings"
   action.

## Prerequisites
- **Plan 03** completed (FilterChip exists, overlay has filter row).
- **Plan 04** completed (Storage panel with clear button exists).

## Steps

### Step 1 — Create Screen/HistoryOverlay/SearchEmpty

Insert into the Main Flow Content frame (`Yj3T1`), after the ClipDetail Wrapper
(`fXtLR`).

```js
({
  operations: [
    { op: "Insert", parent: "Yj3T1", node: {
        type: "frame", name: "SearchEmpty Wrapper",
        layout: "vertical", gap: 12, fill: "#00000000",
        children: [
          { type: "frame", name: "Screen/HistoryOverlay/SearchEmpty",
            layout: "vertical", gap: 12, padding: 16, width: 360,
            cornerRadius: "$radius-xl", fill: "$surface",
            stroke: "$border", strokeWidth: 1,
            effect: { type: "shadow", shadowType: "outer",
                      offset: { x: 0, y: 8 }, blur: 32, color: "#00000030" },
            children: [
              // Search bar with query (no clear button shown for simplicity)
              { type: "frame", name: "searchBar", layout: "horizontal",
                alignItems: "center", gap: 8, padding: [0, 12],
                cornerRadius: "$radius-lg", fill: "$surface",
                stroke: "$border", strokeWidth: 1, height: 36, width: 280,
                children: [
                  { type: "text", name: "icon", content: "search",
                    fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-md" },
                  { type: "text", name: "q", content: "xyznonexistent",
                    fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" }
                ] },
              // Empty state (reuse EmptyState structure inline)
              { type: "frame", name: "emptyState", layout: "vertical",
                alignItems: "center", gap: 12, padding: 24,
                fill: "#00000000",
                children: [
                  { type: "text", name: "icon", content: "search",
                    fill: "$text-muted", fontFamily: "Inter", fontSize: 32 },
                  { type: "text", name: "title", content: "No matching clips",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-lg", fontWeight: "600" },
                  { type: "text", name: "desc", content: "Try a different search term or filter",
                    fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                ] },
              // Shortcut badge
              { type: "frame", name: "badge", layout: "horizontal",
                alignItems: "center", gap: 4, padding: [4, 8],
                cornerRadius: "$radius-sm", fill: "$bg-tertiary",
                children: [
                  { type: "text", name: "txt", content: "Esc to clear search",
                    fill: "$text-secondary", fontFamily: "Inter",
                    fontSize: "$text-sm", fontWeight: "500" }
                ] }
            ] },
          { type: "text", name: "NoteSearchEmpty",
            content: "Search empty state: shown when filter/search yields no clips. Reuses EmptyState visual language. Esc clears search and returns to full history.",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: 11, textGrowth: "fixed-width", width: 320
          }
        ]
    } }
  ]
})
```

### Step 2 — Create Screen/ClearHistoryConfirm

Insert into the Main Flow Content frame (`Yj3T1`). This uses the `DialogFrame`
component (`pHjtP`) as a ref, with the body and footer customized.

> **Note:** The DialogFrame has children `Qrr8` (header), `sXyb2` (body),
> `jWQgA` (footer with a Primary button ref). If ref overrides for nested
> children are not supported, build a standalone dialog that mirrors
> DialogFrame's structure.

**Primary approach (ref with overrides):**
```js
({
  operations: [
    { op: "Insert", parent: "Yj3T1", node: {
        type: "frame", name: "ClearConfirm Wrapper",
        layout: "vertical", gap: 12, fill: "#00000000",
        children: [
          { type: "ref", ref: "pHjtP", name: "Screen/ClearHistoryConfirm",
            overrides: {
              "Qrr8/HiswB/content": "Clear all history?",
              "sXyb2/E5y8zk/content": "This will permanently delete all clipboard history. Pinned items will be kept. This action cannot be undone.",
              "jWQgA/fhZXf/fill": "$danger"
            }
          },
          { type: "text", name: "NoteClearConfirm",
            content: "Clear history confirmation: uses DialogFrame component. Danger button replaces primary. 'Keep pinned items' noted in body. Triggered from Storage panel clear button.",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: 11, textGrowth: "fixed-width", width: 320
          }
        ]
    } }
  ]
})
```

**Fallback (standalone dialog if overrides don't work):**
```js
({
  operations: [
    { op: "Insert", parent: "Yj3T1", node: {
        type: "frame", name: "ClearConfirm Wrapper",
        layout: "vertical", gap: 12, fill: "#00000000",
        children: [
          { type: "frame", name: "Screen/ClearHistoryConfirm",
            layout: "vertical", width: 360,
            cornerRadius: "$radius-xl", fill: "$surface",
            stroke: "$border", strokeWidth: 1,
            effect: { type: "shadow", shadowType: "outer",
                      offset: { x: 0, y: 4 }, blur: 24, color: "#00000020" },
            children: [
              // Header
              { type: "frame", name: "dialogHeader", layout: "horizontal",
                alignItems: "center", gap: 8, padding: 16, width: "fill_container",
                fill: "#00000000",
                children: [
                  { type: "text", name: "title", content: "Clear all history?",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-lg", fontWeight: "600" }
                ] },
              // Body
              { type: "frame", name: "dialogBody", layout: "vertical",
                gap: 12, padding: 16, width: "fill_container", height: 80,
                fill: "#00000000",
                children: [
                  { type: "text", name: "body", content: "This will permanently delete all clipboard history. Pinned items will be kept. This action cannot be undone.",
                    fill: "$text-secondary", fontFamily: "Inter",
                    fontSize: "$text-md", textGrowth: "fixed-width", width: 300 }
                ] },
              // Footer — danger confirm + ghost cancel
              { type: "frame", name: "dialogFooter", layout: "horizontal",
                alignItems: "center", gap: 8, padding: 16, width: "fill_container",
                height: 48, justifyContent: "end", fill: "#00000000",
                children: [
                  { type: "frame", name: "cancelBtn", layout: "horizontal",
                    alignItems: "center", justifyContent: "center", gap: 8,
                    padding: [8, 16], cornerRadius: "$radius-md",
                    fill: "#00000000",
                    children: [
                      { type: "text", name: "t", content: "Cancel",
                        fill: "$accent", fontFamily: "Inter",
                        fontSize: "$text-md", fontWeight: "500" }
                    ] },
                  { type: "frame", name: "confirmBtn", layout: "horizontal",
                    alignItems: "center", justifyContent: "center", gap: 8,
                    padding: [8, 16], cornerRadius: "$radius-md",
                    fill: "$danger",
                    children: [
                      { type: "text", name: "t", content: "Clear history",
                        fill: "#FFFFFF", fontFamily: "Inter",
                        fontSize: "$text-md", fontWeight: "500" }
                    ] }
                ] }
            ] },
          { type: "text", name: "NoteClearConfirm",
            content: "Clear history confirmation: mirrors DialogFrame component. Danger confirm + ghost cancel. Pinned items kept. Triggered from Storage panel.",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: 11, textGrowth: "fixed-width", width: 320
          }
        ]
    } }
  ]
})
```

### Step 3 — Create Screen/PermissionDenied

Insert into the Configuration Content frame (`eCTc4`), after the Appearance
panel.

```js
({
  operations: [
    { op: "Insert", parent: "eCTc4", node: {
        type: "frame", name: "PermissionDenied Wrapper",
        layout: "vertical", gap: 12, fill: "#00000000",
        children: [
          { type: "frame", name: "Screen/PermissionDenied",
            layout: "vertical", alignItems: "center", justifyContent: "center",
            gap: 16, padding: 24, width: 360, height: 400,
            cornerRadius: "$radius-xl", fill: "$surface",
            stroke: "$border", strokeWidth: 1,
            children: [
              // Warning icon
              { type: "text", name: "icon", content: "!",
                fill: "$danger", fontFamily: "Inter",
                fontSize: 48, fontWeight: "700",
                width: 64, height: 64, textAlign: "center",
                textAlignVertical: "middle", textGrowth: "fixed-width-height" },
              // Title
              { type: "text", name: "title", content: "Clipboard Access Denied",
                fill: "$text-primary", fontFamily: "Inter",
                fontSize: "$text-lg", fontWeight: "600",
                textAlign: "center" },
              // Description
              { type: "text", name: "desc",
                content: "Cleepboard needs clipboard permission to capture copied items. Grant access in System Settings > Privacy & Security > Clipboard.",
                fill: "$text-secondary", fontFamily: "Inter",
                fontSize: "$text-md", textAlign: "center",
                textGrowth: "fixed-width", width: 280 },
              // Open System Settings button
              { type: "frame", name: "openSettingsBtn", layout: "horizontal",
                alignItems: "center", justifyContent: "center", gap: 8,
                padding: [8, 16], cornerRadius: "$radius-md",
                fill: "$accent",
                children: [
                  { type: "text", name: "t", content: "Open System Settings",
                    fill: "#FFFFFF", fontFamily: "Inter",
                    fontSize: "$text-md", fontWeight: "500" }
                ] },
              // Secondary link
              { type: "text", name: "laterLink", content: "Maybe later",
                fill: "$text-muted", fontFamily: "Inter",
                fontSize: "$text-sm" }
            ] },
          { type: "text", name: "NotePermissionDenied",
            content: "Permission denied screen (macOS): shown when clipboard access is not granted. Danger warning icon, explanatory text, 'Open System Settings' primary button, 'Maybe later' ghost link. On Windows/Linux this screen is not needed.",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: 11, textGrowth: "fixed-width", width: 320
          }
        ]
    } }
  ]
})
```

### Step 4 — Add edge-state entries to the Behavior Spec

Append a new text node to the Behavior Spec frame (`muaBQ`) documenting the
edge states:

```js
({
  operations: [
    { op: "Insert", parent: "muaBQ", node: {
        type: "text", name: "EdgeStatesSpec",
        content: "Edge States:\n- Search empty: overlay shows EmptyState when no clips match. Esc clears search.\n- Clear confirm: DialogFrame with danger confirm. Pinned items preserved.\n- Permission denied (macOS): warning + 'Open System Settings' button. App runs in read-only mode until granted.\n- Large item truncated: items >10MB show 'Too large to store' badge (see Storage max-size setting).",
        fill: "$text-secondary", fontFamily: "Inter",
        fontSize: 11, textGrowth: "fixed-width", width: 352
    } }
  ]
})
```

## Verification
1. `batch_get` on `Yj3T1` — confirm it now contains HistoryOverlay Wrapper,
   ClipDetail Wrapper, SearchEmpty Wrapper, ClearConfirm Wrapper.
2. `batch_get` on `eCTc4` — confirm it contains the 3 settings panel wrappers
   + PermissionDenied Wrapper.
3. `get_screenshot` on each new screen:
   - **SearchEmpty**: search bar with query, centered empty state, no clip
     cards. 360px width, no overflow.
   - **ClearConfirm**: dialog with "Clear all history?" title, body text, red
     "Clear history" button + "Cancel" ghost button. 360px width.
   - **PermissionDenied**: centered warning icon, title, description, blue
     "Open System Settings" button, "Maybe later" link. 360×400.
4. Confirm the DialogFrame component (`pHjtP`) is now referenced in at least
   one screen (if the ref approach worked) or that the standalone dialog
   mirrors its structure exactly (if fallback was used).

## Commit
```
git add design.pen .plans/
git commit -m "design: add edge states — search-empty, clear-history confirm, permission-denied"
```
