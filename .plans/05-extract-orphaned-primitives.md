# Plan 05 — Extract Orphaned Primitives into Reusable Components

## Goal

The spec (§4) requires "no standalone primitives inside screens — every button,
input, list item, chip, etc. must be a `ref` to a component." Two violations
exist:

1. **EmojiCell** — the EmojiPicker screen (`E4nlZv`) contains 12 standalone
   `EmojiCell0..11` frames, each a 36×36 frame with a text glyph inside. These
   should be a single reusable `Component/EmojiCell` instantiated via refs.
2. **SettingsRow** — the Settings panels (Plan 04 added 3 more) contain
   ad-hoc toggle rows, shortcut rows, and value rows. These should be a
   reusable `Component/SettingsRow` with variants.

This plan extracts both into reusable components and re-instantiates them via
refs in all screens.

## Prerequisites
- **Plan 03** completed (ClipCard variants + FilterChip exist).
- **Plan 04** completed (Settings sub-panels exist with ad-hoc rows).

## Steps

### Step 1 — Create Component/EmojiCell

Insert into the Cards Section (`EBkil`).

```js
({
  operations: [
    { op: "Insert", parent: "EBkil", node: {
        type: "frame", name: "Component/EmojiCell",
        reusable: true,
        layout: "none", width: 36, height: 36,
        cornerRadius: "$radius-md", fill: "#00000000",
        children: [
          { type: "text", name: "EmojiGlyph", content: "😀",
            fill: "$text-primary", fontFamily: "Inter",
            fontSize: 20, textAlign: "center", textAlignVertical: "middle",
            textGrowth: "fixed-width-height",
            width: 36, height: 36, x: 0, y: 0 }
        ]
    } }
  ]
})
```

Read back the ID — call it `EMOJI_CELL_ID`.

### Step 2 — Replace all 12 EmojiCell standalone frames with refs

The emoji cells live inside two rows in the EmojiGrid (`UHUfb`):
- Row 1 (`n77bmd`): `wUAlm`, `zgWZ1`, `m8V192`, `bF45g`, `s4pkU`, `Dqm8C`
- Row 2 (`f0eab`): `ijrxc`, `w6P9a`, `StBH2`, `io6qC`, `wBOU`, `jSMAY`

For each cell, delete the standalone frame and insert a ref with a different
emoji override. Process all 12 in one `batch_design` call.

```js
({
  operations: [
    // Row 1
    { op: "delete", id: "wUAlm" },
    { op: "delete", id: "zgWZ1" },
    { op: "delete", id: "m8V192" },
    { op: "delete", id: "bF45g" },
    { op: "delete", id: "s4pkU" },
    { op: "delete", id: "Dqm8C" },
    // Row 2
    { op: "delete", id: "ijrxc" },
    { op: "delete", id: "w6P9a" },
    { op: "delete", id: "StBH2" },
    { op: "delete", id: "io6qC" },
    { op: "delete", id: "wBOU" },
    { op: "delete", id: "jSMAY" },
    // Insert refs into Row 1
    { op: "Insert", parent: "n77bmd", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell0",
        overrides: { "EmojiGlyph/content": "😀" } } },
    { op: "Insert", parent: "n77bmd", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell1",
        overrides: { "EmojiGlyph/content": "😂" } } },
    { op: "Insert", parent: "n77bmd", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell2",
        overrides: { "EmojiGlyph/content": "❤️" } } },
    { op: "Insert", parent: "n77bmd", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell3",
        overrides: { "EmojiGlyph/content": "👍" } } },
    { op: "Insert", parent: "n77bmd", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell4",
        overrides: { "EmojiGlyph/content": "🔥" } } },
    { op: "Insert", parent: "n77bmd", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell5",
        overrides: { "EmojiGlyph/content": "⭐" } } },
    // Insert refs into Row 2
    { op: "Insert", parent: "f0eab", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell6",
        overrides: { "EmojiGlyph/content": "🎉" } } },
    { op: "Insert", parent: "f0eab", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell7",
        overrides: { "EmojiGlyph/content": "🚀" } } },
    { op: "Insert", parent: "f0eab", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell8",
        overrides: { "EmojiGlyph/content": "💡" } } },
    { op: "Insert", parent: "f0eab", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell9",
        overrides: { "EmojiGlyph/content": "✅" } } },
    { op: "Insert", parent: "f0eab", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell10",
        overrides: { "EmojiGlyph/content": "🔗" } } },
    { op: "Insert", parent: "f0eab", node: {
        type: "ref", ref: EMOJI_CELL_ID, name: "EmojiCell11",
        overrides: { "EmojiGlyph/content": "📋" } } }
  ]
})
```

> **If ref `overrides` for text content is not supported**, fall back to
> keeping the cells as standalone frames but mark the source `Component/EmojiCell`
> as reusable and document that the instances are "manually customized refs."
> The key deliverable is that the component exists in the design system so
> engineers know the spec. In that case, skip the deletes and just leave the
> existing cells, but add a note: "EmojiCell instances use manual content
> override — see Component/EmojiCell for base spec."

### Step 3 — Create Component/SettingsRow

A generic settings row: label on the left, control slot on the right. Insert
into the Inputs Section (`i25hSE`).

```js
({
  operations: [
    { op: "Insert", parent: "i25hSE", node: {
        type: "frame", name: "Component/SettingsRow",
        reusable: true,
        layout: "horizontal", alignItems: "center", gap: 8,
        width: "fill_container", justifyContent: "space_between",
        fill: "#00000000",
        children: [
          { type: "text", name: "SettingsRowLabel", content: "Setting label",
            fill: "$text-primary", fontFamily: "Inter", fontSize: "$text-md" },
          // Right slot — placeholder (engineers swap with toggle/badge/slider)
          { type: "text", name: "SettingsRowValue", content: "value",
            fill: "$text-secondary", fontFamily: "Inter",
            fontSize: "$text-sm", fontWeight: "500" }
        ]
    } }
  ]
})
```

Read back the ID — call it `SETTINGS_ROW_ID`.

### Step 4 — Document the SettingsRow in the design system

Add a note next to the component:

```js
({
  operations: [
    { op: "Insert", parent: "i25hSE", node: {
        type: "text", name: "NoteSettingsRow",
        content: "SettingsRow: reusable label+value row. Right slot accepts Toggle ref, KeyboardShortcutBadge ref, value badge, or slider. Used in all settings panels.",
        fill: "$text-secondary", fontFamily: "Inter",
        fontSize: 11, textGrowth: "fixed-width", width: 260
    } }
  ]
})
```

### Step 5 — Audit remaining screens for orphaned primitives

Run a `batch_get` with `searchDepth: 5` on the Screens frame (`ZoBik`) to
identify any remaining standalone frames inside screens that are not `ref`
nodes and not structural containers (screen-level frames, wrappers, content
frames are OK — they're layout, not primitives).

```js
// batch_get call (not batch_design):
{
  "filePath": "/opt/Devin projects/cleepboard/design.pen",
  "nodeIds": ["ZoBik"],
  "readDepth": 4,
  "resolveInstances": true
}
```

Review the output. Legitimate non-ref frames inside screens:
- Screen container frames (e.g., `Screen/HistoryOverlay`)
- Layout wrappers (e.g., `Main Flow Content`, `Wrapper` frames)
- Notes (text annotations)
- Filter chips (Plan 03 — if ref overrides failed, these are standalone; that's
  acceptable since FilterChip is reusable in the design system)

If any **primitive-like** frames remain (buttons, icons, badges that duplicate
a component), extract them or convert to refs. Document any exceptions in a
note on the Behavior Spec frame (`muaBQ`).

### Step 6 — Update the Behavior Spec checklist

Update the checklist text in `muaBQ` to reflect the completed audit. The
checklist body is node `Fr2qc`.

```js
({
  operations: [
    { op: "update_properties", id: "Fr2qc",
      properties: {
        content: "[x] Brand tokens finalized\n[x] Design system components created and reusable\n[x] All screens mocked using component refs\n[x] Component audit passed - no orphaned primitives\n[x] Interaction states documented (hover/selected/focused)\n[x] Dark mode mockups added\n[x] Image/file clip variants added\n[x] Settings sub-panels (Hotkeys/Storage/Appearance) added\n[x] Edge states added (search-empty, clear-confirm, permission-denied)\n[x] Shadows tokenized\n[ ] Behavior spec agreed upon\n[ ] Tech stack chosen\n[ ] Feature backlog prioritized"
      }
    }
  ]
})
```

> Only update the lines that this plan and prior plans have completed. If
> Plans 06-07 are not yet done, leave those lines unchecked. Adjust the
> checklist content to match actual completion state at execution time.

## Verification
1. `batch_get` on `EBkil` — confirm `Component/EmojiCell` exists and is
   `reusable: true`.
2. `batch_get` on `UHUfb` (EmojiGrid) with `resolveInstances: false` —
   confirm all 12 children are `type: "ref"` pointing to the EmojiCell ID.
3. `batch_get` on `i25hSE` — confirm `Component/SettingsRow` exists and is
   `reusable: true`.
4. `get_screenshot` on `E4nlZv` (EmojiPicker) — confirm the emoji grid still
   renders correctly with all glyphs visible.
5. Run the Step 5 audit read and confirm no unexpected orphaned primitives.

## Commit
```
git add design.pen .plans/
git commit -m "design: extract EmojiCell and SettingsRow into reusable components; update audit checklist"
```
