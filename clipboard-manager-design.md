# Design-First Cross-Platform Clipboard Manager

Create a clean, Apple-inspired brand book and polished UI mockups in `design.pen` for a lightweight, cross-platform clipboard manager before writing any code.

## 1. Visual Identity & Design Tokens
- Define color palette (soft neutrals, subtle grays, one calm accent)
- Typography scale (system fonts for native feel)
- Spacing, radius, shadow, and elevation rules
- Iconography style (SF Symbols / Lucide-like, outlined, minimal)
- Light and dark mode definitions
- Store all tokens as reusable variables in `design.pen` so they propagate to components and screens

## 2. Design System Components (reusable, `reusable: true`)
Build every UI primitive as a reusable component **before** any screen mockups. This is the foundation — all screens below must reference these via `ref` nodes so changes cascade.

### Core Components
- **Button / IconButton** — primary, secondary, ghost, danger; with hover & pressed variants
- **Input / SearchBar** — with focus state, clear button, placeholder
- **ListItem / ClipCard** — text clip, image clip, file clip; hover, selected, active states
- **PinChip** — small pinned indicator badge
- **SectionHeader** — divider with label (e.g., "Pinned", "Today")
- **EmptyState** — icon + message for first-run / no results
- **Tooltip** — small label for hotkey hints or actions
- **Checkbox / Toggle** — for settings switches
- **KeyboardShortcutBadge** — e.g., `Ctrl+Shift+V` styled pill
- **Modal / DialogFrame** — consistent frame for overlays and confirmations
- **ScrollableList** — container with scroll shadows, supporting variable-height rows
- **NavItem / TabItem** — for settings categories or tray menu entries

Each component must include:
- Light and dark theme variants (using variables)
- At least one annotated callout for padding, sizing, and interaction notes
- A frame name prefixed with `Component/` for easy discovery

## 3. Screen Inventory & User Flows
List every state and surface the app needs:
- **History Overlay** — the main popup triggered by a global hotkey, showing recent clips
- **Pinned Section** — persistent items at the top of the history
- **Emoji / Symbol Picker** — grid picker similar to Win+V
- **Clip Detail / Preview** — expanded view for images or long text
- **Settings Panel** — hotkeys, retention policy, theme, startup behavior
- **System Tray / Menu Bar Menu** — quick access when the app is backgrounded
- **Onboarding / Empty State** — first-run experience

## 4. Polished Mockups in `design.pen` (built from component refs)
Build annotated screen mockups inside `design.pen` **exclusively by instantiating the design system components** via `ref` nodes:
- **No standalone primitives** inside screens — every button, input, list item, chip, etc. must be a `ref` to a component from Section 2
- Use the brand tokens from Step 1 (colors, typography, spacing variables)
- Render each screen at 1:1 scale with realistic content
- Show hover/focus/selected states as separate frames or annotated variants
- Keep the file self-contained and viewable in a browser
- Add small callouts for dimensions, padding, and interaction notes
- If a screen needs a new element, first extract it as a new reusable component, then `ref` it in

## 5. Component Maintenance & Updates
- When iterating, edit the source component frame in the design system section — all screen instances update automatically
- Keep a changelog of component versions (e.g., `ClipCard v1.2`)
- Periodically audit screens to ensure no orphaned primitives exist outside the design system

## 6. Cross-Platform Behavior Spec
Document how the app should feel native on each OS:
- **Invocation:** global hotkey (e.g., `Ctrl+Shift+V` / `Cmd+Shift+V`) opens the overlay near the cursor or screen center
- **Dismissal:** `Esc`, clicking outside, or pressing the hotkey again
- **Auto-paste:** selecting an item immediately pastes it at the cursor (configurable)
- **Tray integration:** system tray on Windows/Linux, menu bar on macOS
- **Window chrome:** frameless overlay with OS-appropriate shadows and rounded corners

## 7. Tech Stack Recommendation (for later)
Evaluate and recommend a lightweight stack (e.g., **Tauri** + Rust backend + Web frontend) versus Electron or Flutter, with a short rationale focused on:
- Binary size and memory footprint
- Native API access (global hotkeys, clipboard monitoring, tray icons)
- Cross-platform packaging

## 8. Handoff Checklist
Once the user approves the design, this checklist triggers the coding phase:
- [ ] Brand tokens finalized
- [ ] Design system components created and reusable (`reusable: true`)
- [ ] All screens mocked and annotated using only component `ref` instances
- [ ] Component audit passed — no orphaned primitives in screens
- [ ] Behavior spec agreed upon
- [ ] Tech stack chosen
- [ ] Feature backlog prioritized (history, pinning, emojis, search, etc.)
