# Cleepboard Design Completion Plans

## Purpose

`design.pen` is a clipboard-manager wireframe that is ~60% complete. These
plans close every gap identified in the audit, end-to-end, so the file becomes
a clean engineering handoff. Each plan is self-contained and small enough that
a single AI chat session can execute it without exceeding ~150k tokens of
context.

## Execution Order

Plans **must** be executed in numbered order — later plans depend on nodes
created by earlier ones.

| # | File | Goal | Depends on |
|---|------|------|------------|
| 01 | `01-interaction-states.md` | Hover / selected / focused state frames for Button, ClipCard, SearchBar, NavItem | — |
| 02 | `02-dark-mode.md` | Dark-theme mockups of HistoryOverlay + SettingsPanel using existing `-dark` tokens | 01 |
| 03 | `03-clip-variants-and-focus.md` | Image-thumbnail and file-clip ClipCard variants; keyboard-focused row state in overlay | 01 |
| 04 | `04-settings-subpanels.md` | Hotkeys / Storage / Appearance settings panels (only General exists today) | 01 |
| 05 | `05-extract-orphaned-primitives.md` | Extract EmojiCell + SettingsRow into reusable components; re-instantiate via refs | 03, 04 |
| 06 | `06-edge-states.md` | Search-empty, clear-history confirm dialog, permission-denied screen | 03, 04 |
| 07 | `07-tokenize-shadows-and-icons.md` | Replace inline shadows with `$shadow-*` variables; add icon-font system | 05, 06 |

## Shared Context (applies to every plan)

### File
- **Pen file:** `/opt/Devin projects/cleepboard/design.pen`
- **MCP server:** `pencil` — always call `get_editor_state` first to confirm the
  transport is alive before any `batch_design` / `batch_get` call.

### Top-level frames
| ID | Name |
|----|------|
| `W7zt6X` | Design System |
| `ZoBik` | Screens |
| `muaBQ` | Behavior Spec |

### Design-system section frames (children of `W7zt6X`)
| ID | Name |
|----|------|
| `a6mtU` | Foundation & Actions (contains Tokens, Buttons, Inputs sections) |
| `ymokX` | Content (contains Cards section) |
| `BKMR4` | Overlays (contains Feedback, Navigation sections) |

### Existing reusable components (16)
| ID | Name |
|----|------|
| `WqLbr` | Component/Button/Primary |
| `y1vD4` | Component/Button/Secondary |
| `etzbt` | Component/Button/Ghost |
| `Ffkb1` | Component/Button/Danger |
| `drhvk` | Component/IconButton |
| `wr91x` | Component/SearchBar |
| `i7EFk` | Component/Toggle |
| `G20mW` | Component/PinChip |
| `d7lx2E` | Component/SectionHeader |
| `y4kMU` | Component/ClipCard |
| `F4th3D` | Component/EmptyState |
| `XnID9` | Component/ScrollableList |
| `hZH2R` | Component/Tooltip |
| `GAAmO` | Component/KeyboardShortcutBadge |
| `pHjtP` | Component/DialogFrame |
| `HeYK1` | Component/NavItem |

### Existing screens
| ID | Name | Width |
|----|------|-------|
| `Qj3Ub` | Screen/HistoryOverlay | 360 |
| `I0lwcw` | Screen/ClipDetail | 480 |
| `E4nlZv` | Screen/EmojiPicker | 360 |
| `L3WX9` | Screen/TrayMenu | 200 |
| `a6SspH` | Screen/Onboarding | 360 |
| `sfvQi` | Screen/SettingsPanel | 560 |

### Variables (tokens)
**Colors (light):** `$accent` `#3B82F6`, `$bg-primary` `#FFFFFF`, `$bg-secondary`
`#F8F9FA`, `$bg-tertiary` `#E9ECEF`, `$border` `#DEE2E6`, `$danger` `#FA5252`,
`$success` `#40C057`, `$surface` `#FFFFFF`, `$text-primary` `#212529`,
`$text-secondary` `#495057`, `$text-muted` `#868E96`.

**Colors (dark — currently unused, Plans 02 + 07 wire these up):** `$accent-dark`
`#60A5FA`, `$bg-primary-dark` `#1A1D21`, `$bg-secondary-dark` `#252A2E`,
`$bg-tertiary-dark` `#2F353A`, `$border-dark` `#3A4047`, `$surface-dark`
`#1E2127`, `$text-primary-dark` `#F1F3F5`, `$text-secondary-dark` `#C1C7CD`,
`$text-muted-dark` `#868E96`.

**Radii:** `$radius-sm` 4, `$radius-md` 8, `$radius-lg` 12, `$radius-xl` 16.
**Spacing:** `$spacing-xs` 4, `$spacing-sm` 8, `$spacing-md` 12, `$spacing-lg`
16, `$spacing-xl` 24.
**Type scale:** `$text-sm` 12, `$text-md` 14, `$text-lg` 16, `$text-xl` 20.
**Fonts:** `$font-sans` (system stack), `$font-mono` (SF Mono / JetBrains Mono).

### batch_design rules (from pencil-mcp skill)
- `batch_design` evaluates a **JavaScript expression string** — not natural
  language, not multi-statement scripts.
- No `const` / `let` / `var` declarations.
- Top-level parent for `Insert` is the string `"document"`, never `null`.
- `alignItems` only accepts `"start"`, `"center"`, or `"end"`.
- Only `frame` and `group` nodes can contain `children`.
- Always pass `filePath` explicitly.
- Read the current schema via `get_editor_state(include_schema: true)` once at
  the start of the session if not already cached.

### batch_get rules
- Combine multiple node-ID reads and pattern searches into one call.
- Use `resolveInstances: true` to see the full expanded structure of `ref`
  nodes (shadow-node IDs become `parentId/childId` paths and can be customized
  via `update_properties`).
- Keep `readDepth` ≤ 3 to avoid context overflow.

### Verification (every plan)
After all edits in a plan are applied:
1. Call `batch_get` on every new/modified top-level node with `readDepth: 2` to
   confirm structure is correct.
2. Call `get_screenshot` on each new screen frame and visually verify no
   broken / collapsed / overflowing layout.
3. If a screenshot looks wrong, read the offending node at `readDepth: 3`,
   diagnose, and fix with a targeted `batch_design` update.

### Committing
After each plan is verified, commit with:
```
git add design.pen .plans/
git commit -m "design: <plan title> — <one-line summary>"
```
