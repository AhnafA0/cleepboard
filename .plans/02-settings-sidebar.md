# Plan 02 — Settings sidebar + NavItem architecture + launch-on-login

## Goal
Restructure the Settings view from a single flat scrolling column into the
sidebar + content layout the mockups specify, using a `NavItem` component.
Also surface the `launch_on_login` setting that already exists in the Rust
`Settings` struct but has no UI control.

## Design reference (`design.pen`)
- Screen: `Screen/SettingsPanel` (`sfvQi`)
  - `SettingsSidebar` (`jmNbK`) with `SettingsNav1/2/3` (`NavItem` refs)
  - `SettingsContent` (`bEhT2`) with title, toggle rows, shortcut badge
- Component: `Component/NavItem` (`HeYK1`) — icon + label, with
  `NavItem/Hover` and `NavItem/Active` states (see States Section)
- Dark variant: `Screen/SettingsPanel/Dark` (`GuIFi`) — sidebar uses
  `bg-secondary-dark`, active nav has `accent-dark` left accent.
- Annotation `NoteSettings`:
  > Categories General/Hotkeys/Storage/Appearance. Auto-paste toggle, startup
  > behavior, retention policy, theme selection. All settings persist instantly.

> **Stroke rule note:** the design's `NavItem/Active` historically used a
> left-only accent border, but the governing rule in `clipboard-manager-design.md`
> (commit `f35432c`) mandates **uniform** strokes for focus/active emphasis.
> Implement the active nav as a uniform accent stroke (or a filled accent
> background) — NOT a left-only border.

## Files to touch
- `src/index.html` — restructure `#view-settings` into sidebar + panels
- `src/styles.css` — `.settings-layout`, `.settings-sidebar`, `.nav-item`, panels
- `src/main.js` — nav switching logic, `launch_on_login` wiring

## Proposed categories (mapping existing rows to the design's 4 categories)
- **General** — auto-paste toggle, launch-on-login toggle (new), theme select
- **Hotkeys** — global hotkey badge, register/unregister button + status
- **Storage** — history limit, data directory + move, clear history (danger)
- **Appearance** — theme could live here instead of General; pick one.
  Recommendation: put **Theme + launch-on-login** under General, and keep
  Appearance for future font-size / density controls (stub for now).

## Implementation steps

### 1. HTML — restructure `#view-settings`
Replace the current flat `<section>` blocks with:
```html
<main id="view-settings" class="view settings">
  <div class="settings-layout">
    <nav class="settings-sidebar">
      <button class="nav-item active" data-panel="general">
        <svg>…gear icon…</svg><span>General</span>
      </button>
      <button class="nav-item" data-panel="hotkeys">…</button>
      <button class="nav-item" data-panel="storage">…</button>
      <button class="nav-item" data-panel="appearance">…</button>
    </nav>
    <div class="settings-content">
      <section id="panel-general" class="settings-panel active"> …rows… </section>
      <section id="panel-hotkeys" class="settings-panel"> …rows… </section>
      <section id="panel-storage" class="settings-panel"> …rows… </section>
      <section id="panel-appearance" class="settings-panel"> …rows… </section>
    </div>
  </div>
</main>
```
Move existing setting rows into the appropriate panels. Add the new
launch-on-login row under General:
```html
<div class="setting-row">
  <div><p class="setting-label">Launch on login</p>
       <p class="setting-desc">Start Cleepboard in the background on sign-in.</p></div>
  <label class="toggle"><input id="launch-login" type="checkbox" /><span class="track"></span></label>
</div>
```

### 2. CSS
```css
.settings-layout { display: flex; gap: var(--space-lg); flex: 1; min-height: 0; }
.settings-sidebar {
  width: 152px; flex-shrink: 0; display: flex; flex-direction: column;
  gap: 2px; background: var(--bg-secondary); border-radius: var(--radius-lg);
  padding: var(--space-xs); overflow-y: auto;
}
.nav-item {
  display: flex; align-items: center; gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md); border: none; background: transparent;
  border-radius: var(--radius-md); cursor: pointer; font: inherit; font-weight: 500;
  color: var(--text-secondary); transition: background 0.12s, color 0.12s;
}
.nav-item svg { width: 16px; height: 16px; stroke: currentColor; fill: none; stroke-width: 2; }
.nav-item:hover { background: var(--bg-tertiary); color: var(--text-primary); }
.nav-item.active { background: var(--accent); color: #fff; } /* uniform emphasis, not left border */
.settings-content { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: var(--space-lg); }
.settings-panel { display: none; flex-direction: column; gap: var(--space-md); }
.settings-panel.active { display: flex; }
```
Keep existing `.setting-group`, `.setting-row`, `.toggle`, `.select`, etc.

### 3. JS
- Nav switching (mirror the existing `switchView` pattern):
  ```js
  document.querySelectorAll(".nav-item").forEach((n) =>
    n.addEventListener("click", () => {
      const p = n.dataset.panel;
      document.querySelectorAll(".nav-item").forEach((x) => x.classList.toggle("active", x === n));
      document.querySelectorAll(".settings-panel").forEach((x) => x.classList.toggle("active", x.id === "panel-" + p));
    }));
  ```
- Wire `#launch-login`:
  ```js
  $("#launch-login").addEventListener("change", (e) => {
    settings.launch_on_login = e.target.checked; saveSettings();
  });
  ```
  Add `$("#launch-login").checked = settings.launch_on_login;` to `loadSettings()`
  (src/main.js:264) and to the `settings-updated` listener (src/main.js:421).
- **Backend wiring for launch-on-login is OUT OF SCOPE here** — `Settings` already
  stores the flag; actually registering a GNOME autostart `.desktop` entry is a
  separate backend task. Note it as a follow-up. The toggle should be disabled
  with a tooltip "Coming soon" if you don't want to ship a non-functional control,
  OR ship it as a persisted-only flag and implement the autostart in a follow-up.

## Verification
1. `npm run dev` → Settings tab.
2. Each nav item switches the visible panel; active nav shows accent fill.
3. All existing settings still persist (auto-paste, theme, max-history, data-dir).
4. Launch-on-login toggle reflects and persists the saved value.
5. Dark mode: sidebar uses `bg-secondary-dark`, active nav uses `accent-dark`
   (the CSS variables already swap, so this should work for free).
6. Window min height (300px) — sidebar + content must not overflow/clipped.
   Test by resizing the window to minimum.
