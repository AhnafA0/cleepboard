# Plan 04 — Settings Sub-Panels (Hotkeys / Storage / Appearance)

## Goal

The spec (§3) lists Settings Panel categories: General, Hotkeys, Storage,
Appearance. Today only **General** is mocked (inside `bEhT2` SettingsContent).
The sidebar (`jmNbK`) has 3 NavItems all labeled "General". This plan:
1. Fixes the sidebar labels to read General / Hotkeys / Storage / Appearance.
2. Creates three new settings content frames (Hotkeys, Storage, Appearance).
3. Places them in the Screens frame as separate annotated mockups so each
   panel is visible without tab-switching.

## Prerequisites
- **Plan 01** completed (NavItem active state visual language exists).

## Current structure
- `sfvQi` = Screen/SettingsPanel (560×400, horizontal layout)
  - `jmNbK` = SettingsSidebar (180px, vertical, 3 NavItem refs: `IGxvw`,
    `ZowhV`, `d7Kwch` — all say "General")
  - `bEhT2` = SettingsContent (vertical, contains General title + 2 toggle
    rows + shortcut label + badge)

## Steps

### Step 1 — Fix sidebar NavItem labels

Update the three existing NavItem refs and add a fourth. The NavItem refs are
`IGxvw`, `ZowhV`, `d7Kwch`. We need to override their labels. Since they are
refs to `HeYK1`, use `update_properties` on the shadow-node text paths.

```js
({
  operations: [
    { op: "update_properties", id: "IGxvw/KkQP1",
      properties: { content: "General" } },
    { op: "update_properties", id: "IGxvw/Jn1V2",
      properties: { content: "settings" } },
    { op: "update_properties", id: "ZowhV/KkQP1",
      properties: { content: "Hotkeys" } },
    { op: "update_properties", id: "ZowhV/Jn1V2",
      properties: { content: "keyboard" } },
    { op: "update_properties", id: "d7Kwch/KkQP1",
      properties: { content: "Storage" } },
    { op: "update_properties", id: "d7Kwch/Jn1V2",
      properties: { content: "database" } },
    // Add 4th nav item: Appearance
    { op: "Insert", parent: "jmNbK", node: {
        type: "ref", ref: "HeYK1", name: "SettingsNav4",
        overrides: {
          "Jn1V2/content": "palette",
          "KkQP1/content": "Appearance"
        }
      }
    }
  ]
})
```

> If `overrides` on ref Insert is not supported, create a standalone NavItem
> frame instead:
> ```js
> { op: "Insert", parent: "jmNbK", node: {
>     type: "frame", name: "SettingsNav4", layout: "horizontal",
>     alignItems: "center", gap: 8, padding: [0, 12],
>     cornerRadius: "$radius-md", fill: "#00000000",
>     height: 36, width: 240,
>     children: [
>       { type: "text", name: "NavItemIcon", content: "palette",
>         fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
>       { type: "text", name: "NavItemLabel", content: "Appearance",
>         fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
>     ]
>   }
> }
> ```

### Step 2 — Create Screen/SettingsHotkeys

Insert a new standalone screen in the Screens frame (`ZoBik`), inside the
Configuration category (`WR55B`), in the Config Content frame (`eCTc4`).

This panel shows a hotkey recorder UI: each action has a label and a
keyboard-shortcut badge that's in a "recording" state for one of them.

```js
({
  operations: [
    { op: "Insert", parent: "eCTc4", node: {
        type: "frame", name: "HotkeysPanel Wrapper",
        layout: "vertical", gap: 12, fill: "#00000000",
        children: [
          { type: "frame", name: "Screen/SettingsHotkeys",
            layout: "horizontal", width: 560, height: 400,
            cornerRadius: "$radius-xl", fill: "$surface",
            stroke: "$border", strokeWidth: 1,
            children: [
              // Reuse sidebar visual (standalone, not ref, to show Hotkeys active)
              { type: "frame", name: "HotkeysSidebar",
                layout: "vertical", gap: 4, padding: 16, width: 180,
                height: "fill_container", cornerRadius: [16, 0, 0, 16],
                fill: "$bg-secondary",
                children: [
                  { type: "frame", name: "n1", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "settings",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "General",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] },
                  { type: "frame", name: "n2", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "$bg-tertiary",
                    height: 36, width: 240,
                    stroke: "$accent", strokeWidth: { left: 3, top: 0, right: 0, bottom: 0 },
                    strokeAlignment: "inner",
                    children: [
                      { type: "text", name: "i", content: "keyboard",
                        fill: "$accent", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Hotkeys",
                        fill: "$accent", fontFamily: "Inter",
                        fontSize: "$text-md", fontWeight: "600" }
                    ] },
                  { type: "frame", name: "n3", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "database",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Storage",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] },
                  { type: "frame", name: "n4", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "palette",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Appearance",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] }
                ] },
              // Content
              { type: "frame", name: "HotkeysContent",
                layout: "vertical", gap: 16, padding: 24, width: "fill_container",
                fill: "#00000000",
                children: [
                  { type: "text", name: "title", content: "Hotkeys",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-xl", fontWeight: "700" },
                  // Row: Open history
                  { type: "frame", name: "hkRow1", layout: "horizontal",
                    alignItems: "center", gap: 8, width: "fill_container",
                    justifyContent: "space_between", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Open history overlay",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      { type: "frame", name: "badge", layout: "horizontal",
                        alignItems: "center", gap: 4, padding: [4, 8],
                        cornerRadius: "$radius-sm", fill: "$bg-tertiary",
                        children: [
                          { type: "text", name: "txt", content: "Ctrl+Shift+V",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] }
                    ] },
                  // Row: Open emoji picker
                  { type: "frame", name: "hkRow2", layout: "horizontal",
                    alignItems: "center", gap: 8, width: "fill_container",
                    justifyContent: "space_between", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Open emoji picker",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      { type: "frame", name: "badge", layout: "horizontal",
                        alignItems: "center", gap: 4, padding: [4, 8],
                        cornerRadius: "$radius-sm", fill: "$bg-tertiary",
                        children: [
                          { type: "text", name: "txt", content: "Ctrl+Shift+E",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] }
                    ] },
                  // Row: Toggle pin (recording state)
                  { type: "frame", name: "hkRow3", layout: "horizontal",
                    alignItems: "center", gap: 8, width: "fill_container",
                    justifyContent: "space_between", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Toggle pin on selected",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      // Recording badge — accent border, dashed feel
                      { type: "frame", name: "recordingBadge", layout: "horizontal",
                        alignItems: "center", gap: 4, padding: [4, 8],
                        cornerRadius: "$radius-sm", fill: "#00000000",
                        stroke: "$accent", strokeWidth: 2,
                        children: [
                          { type: "text", name: "txt", content: "Press keys...",
                            fill: "$accent", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] }
                    ] },
                  // Row: Clear history
                  { type: "frame", name: "hkRow4", layout: "horizontal",
                    alignItems: "center", gap: 8, width: "fill_container",
                    justifyContent: "space_between", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Clear all history",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      { type: "frame", name: "badge", layout: "horizontal",
                        alignItems: "center", gap: 4, padding: [4, 8],
                        cornerRadius: "$radius-sm", fill: "$bg-tertiary",
                        children: [
                          { type: "text", name: "txt", content: "Ctrl+Shift+D",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] }
                    ] },
                  // Hint
                  { type: "text", name: "hint", content: "Click any shortcut to rebind. Esc cancels recording.",
                    fill: "$text-muted", fontFamily: "Inter", fontSize: "$text-sm" }
                ] }
            ]
        },
        { type: "text", name: "NoteHotkeys",
          content: "Hotkeys panel: each action shows current shortcut badge. Click to enter recording mode (accent outline, 'Press keys...' text). Esc cancels. Conflicts highlighted in red.",
          fill: "$text-secondary", fontFamily: "Inter",
          fontSize: 11, textGrowth: "fixed-width", width: 481
        }
      ]
    } }
  ]
})
```

### Step 3 — Create Screen/SettingsStorage

```js
({
  operations: [
    { op: "Insert", parent: "eCTc4", node: {
        type: "frame", name: "StoragePanel Wrapper",
        layout: "vertical", gap: 12, fill: "#00000000",
        children: [
          { type: "frame", name: "Screen/SettingsStorage",
            layout: "horizontal", width: 560, height: 400,
            cornerRadius: "$radius-xl", fill: "$surface",
            stroke: "$border", strokeWidth: 1,
            children: [
              // Sidebar (Storage active)
              { type: "frame", name: "StorageSidebar",
                layout: "vertical", gap: 4, padding: 16, width: 180,
                height: "fill_container", cornerRadius: [16, 0, 0, 16],
                fill: "$bg-secondary",
                children: [
                  { type: "frame", name: "n1", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "settings",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "General",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] },
                  { type: "frame", name: "n2", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "keyboard",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Hotkeys",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] },
                  { type: "frame", name: "n3", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "$bg-tertiary",
                    height: 36, width: 240,
                    stroke: "$accent", strokeWidth: { left: 3, top: 0, right: 0, bottom: 0 },
                    strokeAlignment: "inner",
                    children: [
                      { type: "text", name: "i", content: "database",
                        fill: "$accent", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Storage",
                        fill: "$accent", fontFamily: "Inter",
                        fontSize: "$text-md", fontWeight: "600" }
                    ] },
                  { type: "frame", name: "n4", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "palette",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Appearance",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] }
                ] },
              // Content
              { type: "frame", name: "StorageContent",
                layout: "vertical", gap: 16, padding: 24, width: "fill_container",
                fill: "#00000000",
                children: [
                  { type: "text", name: "title", content: "Storage",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-xl", fontWeight: "700" },
                  // Retention: keep last N items
                  { type: "frame", name: "retentionRow", layout: "horizontal",
                    alignItems: "center", gap: 8, width: "fill_container",
                    justifyContent: "space_between", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Keep last",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      { type: "frame", name: "valueBadge", layout: "horizontal",
                        alignItems: "center", gap: 4, padding: [4, 12],
                        cornerRadius: "$radius-sm", fill: "$bg-tertiary",
                        children: [
                          { type: "text", name: "v", content: "500 items",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] }
                    ] },
                  // Expire after X days
                  { type: "frame", name: "expireRow", layout: "horizontal",
                    alignItems: "center", gap: 8, width: "fill_container",
                    justifyContent: "space_between", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Expire after",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      { type: "frame", name: "valueBadge", layout: "horizontal",
                        alignItems: "center", gap: 4, padding: [4, 12],
                        cornerRadius: "$radius-sm", fill: "$bg-tertiary",
                        children: [
                          { type: "text", name: "v", content: "30 days",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] }
                    ] },
                  // Max storage size
                  { type: "frame", name: "maxSizeRow", layout: "horizontal",
                    alignItems: "center", gap: 8, width: "fill_container",
                    justifyContent: "space_between", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Max image size",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      { type: "frame", name: "valueBadge", layout: "horizontal",
                        alignItems: "center", gap: 4, padding: [4, 12],
                        cornerRadius: "$radius-sm", fill: "$bg-tertiary",
                        children: [
                          { type: "text", name: "v", content: "10 MB",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] }
                    ] },
                  // Usage bar
                  { type: "frame", name: "usageSection", layout: "vertical",
                    gap: 4, width: "fill_container", fill: "#00000000",
                    children: [
                      { type: "text", name: "usageLabel", content: "Current usage: 142 MB / 500 MB",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-sm" },
                      // Progress bar
                      { type: "frame", name: "usageBar", layout: "horizontal",
                        width: "fill_container", height: 8, cornerRadius: 4,
                        fill: "$bg-tertiary",
                        children: [
                          { type: "rectangle", name: "usageFill",
                            fill: "$accent", cornerRadius: 4,
                            width: 120, height: 8 }
                        ] }
                    ] },
                  // Clear history button (danger)
                  { type: "frame", name: "clearBtn", layout: "horizontal",
                    alignItems: "center", justifyContent: "center", gap: 8,
                    padding: [8, 16], cornerRadius: "$radius-md",
                    fill: "$danger",
                    children: [
                      { type: "text", name: "t", content: "Clear all history",
                        fill: "#FFFFFF", fontFamily: "Inter",
                        fontSize: "$text-md", fontWeight: "500" }
                    ] }
                ] }
            ]
        },
        { type: "text", name: "NoteStorage",
          content: "Storage panel: retention (keep last N, expire after X days), max image size, usage bar, clear-history danger button. Clear triggers confirmation dialog (see Plan 06).",
          fill: "$text-secondary", fontFamily: "Inter",
          fontSize: 11, textGrowth: "fixed-width", width: 481
        }
      ]
    } }
  ]
})
```

### Step 4 — Create Screen/SettingsAppearance

```js
({
  operations: [
    { op: "Insert", parent: "eCTc4", node: {
        type: "frame", name: "AppearancePanel Wrapper",
        layout: "vertical", gap: 12, fill: "#00000000",
        children: [
          { type: "frame", name: "Screen/SettingsAppearance",
            layout: "horizontal", width: 560, height: 400,
            cornerRadius: "$radius-xl", fill: "$surface",
            stroke: "$border", strokeWidth: 1,
            children: [
              // Sidebar (Appearance active)
              { type: "frame", name: "AppearanceSidebar",
                layout: "vertical", gap: 4, padding: 16, width: 180,
                height: "fill_container", cornerRadius: [16, 0, 0, 16],
                fill: "$bg-secondary",
                children: [
                  { type: "frame", name: "n1", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "settings",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "General",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] },
                  { type: "frame", name: "n2", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "keyboard",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Hotkeys",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] },
                  { type: "frame", name: "n3", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "#00000000",
                    height: 36, width: 240,
                    children: [
                      { type: "text", name: "i", content: "database",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Storage",
                        fill: "$text-secondary", fontFamily: "Inter", fontSize: "$text-md" }
                    ] },
                  { type: "frame", name: "n4", layout: "horizontal",
                    alignItems: "center", gap: 8, padding: [0, 12],
                    cornerRadius: "$radius-md", fill: "$bg-tertiary",
                    height: 36, width: 240,
                    stroke: "$accent", strokeWidth: { left: 3, top: 0, right: 0, bottom: 0 },
                    strokeAlignment: "inner",
                    children: [
                      { type: "text", name: "i", content: "palette",
                        fill: "$accent", fontFamily: "Inter", fontSize: 16 },
                      { type: "text", name: "l", content: "Appearance",
                        fill: "$accent", fontFamily: "Inter",
                        fontSize: "$text-md", fontWeight: "600" }
                    ] }
                ] },
              // Content
              { type: "frame", name: "AppearanceContent",
                layout: "vertical", gap: 16, padding: 24, width: "fill_container",
                fill: "#00000000",
                children: [
                  { type: "text", name: "title", content: "Appearance",
                    fill: "$text-primary", fontFamily: "Inter",
                    fontSize: "$text-xl", fontWeight: "700" },
                  // Theme selector — 3 swatches
                  { type: "text", name: "themeLabel", content: "Theme",
                    fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                  { type: "frame", name: "themeRow", layout: "horizontal",
                    gap: 12, fill: "#00000000",
                    children: [
                      // Light (selected)
                      { type: "frame", name: "themeLight", layout: "vertical",
                        gap: 4, alignItems: "center", fill: "#00000000",
                        children: [
                          { type: "rectangle", name: "swatch", fill: "$bg-primary",
                            width: 56, height: 40, cornerRadius: "$radius-md",
                            stroke: "$accent", strokeWidth: 2 },
                          { type: "text", name: "l", content: "Light",
                            fill: "$accent", fontFamily: "Inter",
                            fontSize: "$text-sm", fontWeight: "500" }
                        ] },
                      // Dark
                      { type: "frame", name: "themeDark", layout: "vertical",
                        gap: 4, alignItems: "center", fill: "#00000000",
                        children: [
                          { type: "rectangle", name: "swatch", fill: "$bg-primary-dark",
                            width: 56, height: 40, cornerRadius: "$radius-md",
                            stroke: "$border", strokeWidth: 1 },
                          { type: "text", name: "l", content: "Dark",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm" }
                        ] },
                      // System
                      { type: "frame", name: "themeSystem", layout: "vertical",
                        gap: 4, alignItems: "center", fill: "#00000000",
                        children: [
                          { type: "rectangle", name: "swatch",
                            fill: { type: "gradient", gradientType: "linear",
                              rotation: 90, colors: [
                                { color: "$bg-primary", position: 0 },
                                { color: "$bg-primary-dark", position: 1 }
                              ] },
                            width: 56, height: 40, cornerRadius: "$radius-md",
                            stroke: "$border", strokeWidth: 1 },
                          { type: "text", name: "l", content: "System",
                            fill: "$text-secondary", fontFamily: "Inter",
                            fontSize: "$text-sm" }
                        ] }
                    ] },
                  // Accent color
                  { type: "text", name: "accentLabel", content: "Accent color",
                    fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                  { type: "frame", name: "accentRow", layout: "horizontal",
                    gap: 8, fill: "#00000000",
                    children: [
                      { type: "ellipse", name: "c1", fill: "$accent",
                        width: 24, height: 24,
                        stroke: "$text-primary", strokeWidth: 2 },
                      { type: "ellipse", name: "c2", fill: "#7C3AED",
                        width: 24, height: 24 },
                      { type: "ellipse", name: "c3", fill: "$success",
                        width: 24, height: 24 },
                      { type: "ellipse", name: "c4", fill: "$danger",
                        width: 24, height: 24 },
                      { type: "ellipse", name: "c5", fill: "#F59E0B",
                        width: 24, height: 24 }
                    ] },
                  // Overlay opacity slider (visual)
                  { type: "frame", name: "opacityRow", layout: "vertical",
                    gap: 4, width: "fill_container", fill: "#00000000",
                    children: [
                      { type: "text", name: "label", content: "Overlay opacity: 95%",
                        fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
                      { type: "frame", name: "slider", layout: "horizontal",
                        width: "fill_container", height: 8, cornerRadius: 4,
                        fill: "$bg-tertiary",
                        children: [
                          { type: "rectangle", name: "fill", fill: "$accent",
                            cornerRadius: 4, width: 280, height: 8 }
                        ] }
                    ] }
                ] }
            ]
        },
        { type: "text", name: "NoteAppearance",
          content: "Appearance panel: theme picker (Light/Dark/System swatches), accent color dots, overlay opacity slider. System = gradient swatch indicating auto-switch.",
          fill: "$text-secondary", fontFamily: "Inter",
          fontSize: 11, textGrowth: "fixed-width", width: 481
        }
      ]
    } }
  ]
})
```

## Verification
1. `batch_get` on `jmNbK` — confirm 4 NavItems labeled General / Hotkeys /
   Storage / Appearance.
2. `batch_get` on `eCTc4` — confirm it now contains the original
   SettingsPanel Wrapper + 3 new panel wrappers (Hotkeys, Storage,
   Appearance).
3. `get_screenshot` on each new settings screen — confirm:
   - Sidebar shows correct active state (accent left border on the right
     category).
   - Hotkeys: 4 shortcut rows, one in recording state (accent outline).
   - Storage: retention rows, usage bar (~28% filled), red clear button.
   - Appearance: 3 theme swatches (Light selected with accent border),
     accent color dots, opacity slider.
   - No overflow at 560×400.

## Commit
```
git add design.pen .plans/
git commit -m "design: add Hotkeys, Storage, Appearance settings sub-panels; fix sidebar labels"
```
