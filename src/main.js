const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ===== State =====
let items = [];
let filtered = [];
let selectedIndex = 0;
let currentView = "history";
let activeFilter = "all";
let settings = { auto_paste: true, theme: "system", max_history: 100 };

const URL_RE = /^https?:\/\//;

const $ = (sel) => document.querySelector(sel);
const overlayEl = $("#overlay");
const listEl = $("#list");
const searchEl = $("#search");
const emptyEl = $("#empty");

// ===== Helpers =====
function timeAgo(ts) {
  const s = Math.floor(Date.now() / 1000) - ts;
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} hr ago`;
  return `${Math.floor(s / 86400)} d ago`;
}

function isToday(ts) {
  const d = new Date(ts * 1000);
  const n = new Date();
  return d.toDateString() === n.toDateString();
}

function looksLikeCode(t) {
  return /[{};=<>]|\bfunction\b|\bconst\b|\bimport\b|=>/.test(t) && t.length < 400;
}

function iconFor(item) {
  if (item.kind === "image") {
    return `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="M21 15l-5-5L5 21"/></svg>`;
  }
  if (item.kind === "file") {
    // Framed extension badge (ClipFileIcon). Use the first filename's ext;
    // fall back to "FILE" when there's no extension. Preview is comma-joined
    // for multi-file clips, so isolate the first filename and anchor to its
    // final dot so `report.final.pdf` -> PDF, not FIN.
    const first = (item.preview || "").split(",")[0].trim();
    const m = first.match(/\.(\w+)$/);
    const ext = m ? m[1].toUpperCase().slice(0, 4) : "FILE";
    return `<span class="clip-file-ext">${escapeHtml(ext)}</span>`;
  }
  // `text` is not shipped in the list payload (Store::summaries strips it);
  // the 160-char preview is enough for both link and code detection.
  const t = item.preview || "";
  if (URL_RE.test(t.trim())) {
    return `<svg viewBox="0 0 24 24"><path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1"/><path d="M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1"/></svg>`;
  }
  if (looksLikeCode(t)) {
    return `<svg viewBox="0 0 24 24"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>`;
  }
  return "T";
}

// ===== Rendering =====
// applyFilter is async when a search query is present: full-text matching
// runs backend-side (search_history) because the list payload carries no
// clip text. `filterSeq` discards stale results from earlier keystrokes.
let filterSeq = 0;
async function applyFilter() {
  const seq = ++filterSeq;
  const q = searchEl.value.trim();
  let base;
  if (q) {
    try {
      base = await invoke("search_history", { query: q });
    } catch (_) {
      base = items.slice();
    }
    if (seq !== filterSeq) return; // a newer query already ran
  } else {
    base = items.slice();
  }
  filtered = base;
  if (activeFilter !== "all") {
    filtered = filtered.filter((i) => {
      if (activeFilter === "link")
        return i.kind === "text" && URL_RE.test((i.preview || "").trim());
      if (activeFilter === "file") return i.kind === "file"; // see Plan 03
      return i.kind === activeFilter; // "text" | "image"
    });
  }
  if (selectedIndex >= filtered.length) selectedIndex = Math.max(0, filtered.length - 1);
  renderList();
}

function renderList() {
  listEl.innerHTML = "";
  if (filtered.length === 0) {
    // Distinguish "nothing copied yet" from "search/filter matched nothing".
    const narrowing = searchEl.value.trim() !== "" || activeFilter !== "all";
    $("#empty-title").textContent = narrowing ? "No clips match" : "No clips yet";
    $("#empty-sub").textContent = narrowing
      ? "Try a different search or filter."
      : "Copy something and it'll show up here.";
    emptyEl.classList.remove("hidden");
    listEl.classList.add("hidden");
    return;
  }
  emptyEl.classList.add("hidden");
  listEl.classList.remove("hidden");

  const pinned = filtered.filter((i) => i.pinned);
  const rest = filtered.filter((i) => !i.pinned);

  // id -> position in `filtered`, built once per render (indexOf per row
  // would make rendering O(n²)).
  const indexOf = new Map(filtered.map((it, idx) => [it.id, idx]));

  if (pinned.length) {
    listEl.appendChild(sectionHeader("Pinned", pinned.length));
    pinned.forEach((i) => listEl.appendChild(clipEl(i, indexOf)));
  }
  if (rest.length) {
    const today = rest.filter((i) => isToday(i.timestamp));
    const earlier = rest.filter((i) => !isToday(i.timestamp));
    if (today.length) {
      listEl.appendChild(sectionHeader("Today", today.length));
      today.forEach((i) => listEl.appendChild(clipEl(i, indexOf)));
    }
    if (earlier.length) {
      listEl.appendChild(sectionHeader("Earlier", earlier.length));
      earlier.forEach((i) => listEl.appendChild(clipEl(i, indexOf)));
    }
  }
  highlightSelected();
}

function sectionHeader(label, count) {
  const el = document.createElement("div");
  el.className = "section-header";
  el.innerHTML = `<span class="section-header-text"><span>${label}</span><span class="count">· ${count}</span></span>`;
  return el;
}

function clipEl(item, indexOf) {
  const idx = indexOf.get(item.id);
  const el = document.createElement("div");
  el.className = "clip";
  el.dataset.index = idx;
  el.dataset.id = item.id;

  const icon = iconFor(item);
  const isImg = item.kind === "image";
  const isFile = item.kind === "file";
  // `text` isn't in the list payload — the preview is enough for code detection.
  const textCls = !isFile && looksLikeCode(item.preview || "") ? "clip-text mono" : "clip-text";
  const pin = item.pinned
    ? `<span class="pin-chip"><svg viewBox="0 0 24 24"><path d="M12 2l2 7h7l-5.5 4 2 7L12 16l-5.5 4 2-7L3 9h7z"/></svg>Pinned</span>`
    : "";

  el.innerHTML = `
    <div class="clip-icon${isFile ? " file" : ""}">${icon}</div>
    <div class="clip-body">
      <div class="${textCls}">${escapeHtml(item.preview || "")}</div>
      <div class="clip-meta">${timeAgo(item.timestamp)}${pin}</div>
    </div>
    <div class="clip-actions">
      <button class="icon-btn ${item.pinned ? "active" : ""}" data-act="pin" title="Pin">
        <svg viewBox="0 0 24 24"><path d="M12 2l2 7h7l-5.5 4 2 7L12 16l-5.5 4 2-7L3 9h7z"/></svg>
      </button>
      <button class="icon-btn" data-act="info" title="Details">
        <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
      </button>
      <button class="icon-btn" data-act="del" title="Delete">
        <svg viewBox="0 0 24 24"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
      </button>
    </div>`;

  if (isImg) loadThumb(el.querySelector(".clip-icon"), item.id);

  el.addEventListener("click", (e) => {
    const actBtn = e.target.closest("[data-act]");
    if (actBtn) {
      e.stopPropagation();
      const act = actBtn.dataset.act;
      if (act === "pin") pinItem(item.id);
      else if (act === "del") deleteItem(item.id);
      else if (act === "info") openDetail(item);
      return;
    }
    selectedIndex = idx;
    pasteSelected();
  });
  el.addEventListener("mouseenter", () => {
    selectedIndex = idx;
    highlightSelected(false);
  });
  return el;
}

async function loadThumb(container, id) {
  // Bounded thumbnail — the full PNG only loads in the detail view.
  const url = await invoke("get_image_thumb", { id }).catch(() => null);
  if (url) {
    container.innerHTML = `<img src="${url}" alt="" />`;
  } else {
    // Image file is gone (or unreadable) — muted strike-through icon.
    container.innerHTML = `<svg class="thumb-missing" viewBox="0 0 24 24"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="M21 15l-5-5L5 21"/><line x1="4" y1="20" x2="20" y2="4"/></svg>`;
    container.title = "image unavailable";
  }
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

function highlightSelected(viaKeyboard = false) {
  document.querySelectorAll(".clip").forEach((el) => {
    const on = Number(el.dataset.index) === selectedIndex;
    el.classList.toggle("selected", on);
    // The accent ring marks the keyboard-highlighted row only; mouse hover
    // sets the selected fill without the ring (ClipCard/Focused vs Selected).
    el.classList.toggle("focused", on && viaKeyboard);
    if (on && viaKeyboard) el.scrollIntoView({ block: "nearest" });
  });
}

// ===== Actions =====
async function refresh() {
  items = await invoke("get_history");
  applyFilter();
}

async function pasteSelected() {
  const item = filtered[selectedIndex];
  if (!item) return;
  const ok = await invoke("copy_item", { id: item.id, paste: settings.auto_paste });
  if (!ok) showToast(copyFailMsg());
}

async function pinItem(id) {
  items = await invoke("toggle_pin", { id });
  applyFilter();
}

async function deleteItem(id) {
  items = await invoke("delete_item", { id });
  applyFilter();
}

// ===== Detail modal =====
const detailEl = $("#detail");
let detailItem = null;

async function openDetail(item) {
  detailItem = item;
  const body = $("#detail-body");
  const meta = $("#detail-meta");
  $("#detail-title").textContent =
    item.kind === "image" ? "Image clip" : item.kind === "file" ? "File clip" : "Text clip";
  // Metadata strip: timestamp + source app. Source-app capture is X11-only;
  // on Wayland it is `null` and we show "Unknown" (see Plan 05).
  const time = new Date(item.timestamp * 1000).toLocaleString();
  const src = item.source_app ? escapeHtml(item.source_app) : "Unknown";
  meta.innerHTML = `<span>Copied ${time} <span class="detail-meta-sep">·</span> From ${src}</span>`;
  if (item.kind === "image") {
    const url = await invoke("get_image_data_url", { id: item.id }).catch(() => null);
    body.innerHTML = url
      ? `<img src="${url}" alt="" />`
      : `<p class="detail-unavailable">Image unavailable — the stored file is gone.</p>`;
  } else if (item.kind === "file") {
    // `text` holds the raw `text/uri-list` payload; show one URI per line.
    const full = await invoke("get_item_text", { id: item.id });
    const lines = (full || "").split("\n").filter(Boolean);
    body.innerHTML = `<pre>${escapeHtml(lines.join("\n"))}</pre>`;
  } else {
    const full = await invoke("get_item_text", { id: item.id });
    body.innerHTML = `<pre>${escapeHtml(full || item.preview || "")}</pre>`;
  }
  detailEl.classList.remove("hidden");
}

$("#detail-close").addEventListener("click", () => detailEl.classList.add("hidden"));
$("#detail-copy").addEventListener("click", async () => {
  if (detailItem) {
    const ok = await invoke("copy_item", { id: detailItem.id, paste: settings.auto_paste });
    if (!ok) {
      showToast(copyFailMsg());
      return;
    }
  }
  detailEl.classList.add("hidden");
});
detailEl.addEventListener("click", (e) => {
  if (e.target === detailEl) detailEl.classList.add("hidden");
});

// Backdrop click dismissal: .overlay is the transparent fullscreen layer
// around .panel — a click that lands on it directly (not inside the panel)
// closes the overlay. The detail modal sits above it and stops its own.
overlayEl.addEventListener("click", (e) => {
  if (e.target === overlayEl) invoke("hide_window");
});

// ===== View switching =====
function switchView(view) {
  currentView = view;
  document.querySelectorAll(".tab").forEach((t) => t.classList.toggle("active", t.dataset.view === view));
  document.querySelectorAll(".view").forEach((v) => v.classList.remove("active"));
  $(`#view-${view}`).classList.add("active");
  if (view === "history") {
    searchEl.placeholder = "Search clipboard…";
    searchEl.focus();
  } else if (view === "emoji") {
    searchEl.placeholder = "Search emoji…";
    renderEmoji(searchEl.value);
  } else {
    searchEl.placeholder = "Settings";
  }
}

document.querySelectorAll(".tab").forEach((t) => {
  t.addEventListener("click", () => switchView(t.dataset.view));
});

// ===== Filter chips =====
document.querySelectorAll(".filter-chip").forEach((chip) => {
  chip.addEventListener("click", () => {
    document.querySelectorAll(".filter-chip").forEach((c) => c.classList.remove("active"));
    chip.classList.add("active");
    activeFilter = chip.dataset.filter;
    selectedIndex = 0;
    applyFilter();
  });
});

// ===== Emoji picker =====
const EMOJI = [
  { ch: "😀", name: "grin happy face smile" },
  { ch: "😂", name: "joy laugh cry face" },
  { ch: "😍", name: "heart eyes love face" },
  { ch: "🥰", name: "love hearts face adore" },
  { ch: "😎", name: "cool sunglasses face smile" },
  { ch: "🤔", name: "think ponder face hmm" },
  { ch: "😅", name: "sweat laugh face nervous" },
  { ch: "😭", name: "cry sob tears face sad" },
  { ch: "😡", name: "angry rage face mad" },
  { ch: "👍", name: "thumbs up yes like" },
  { ch: "👎", name: "thumbs down no dislike" },
  { ch: "👏", name: "clap applause hands" },
  { ch: "🙏", name: "pray thanks hands please" },
  { ch: "💪", name: "muscle strong arm flex" },
  { ch: "🔥", name: "fire flame hot lit" },
  { ch: "✨", name: "sparkles shine magic" },
  { ch: "🎉", name: "party celebrate tada" },
  { ch: "❤️", name: "heart love red" },
  { ch: "💔", name: "broken heart sad love" },
  { ch: "⭐", name: "star favorite" },
  { ch: "✅", name: "check mark done yes" },
  { ch: "❌", name: "cross mark no x" },
  { ch: "⚠️", name: "warning caution alert" },
  { ch: "💡", name: "idea light bulb" },
  { ch: "📌", name: "pin pushpin mark" },
  { ch: "📎", name: "paperclip attach" },
  { ch: "✂️", name: "scissors cut" },
  { ch: "📋", name: "clipboard copy paste" },
  { ch: "🔍", name: "search magnify glass" },
  { ch: "🔒", name: "lock locked secure" },
  { ch: "🔓", name: "unlock unlocked open" },
  { ch: "🚀", name: "rocket launch ship" },
  { ch: "🌟", name: "star glow sparkle" },
  { ch: "🐛", name: "bug insect" },
  { ch: "⚡", name: "zap lightning bolt fast" },
  { ch: "💻", name: "computer laptop" },
  { ch: "📱", name: "phone mobile" },
  { ch: "⌨️", name: "keyboard type" },
  { ch: "🖱️", name: "mouse click pointer" },
  { ch: "📁", name: "folder file directory" },
  { ch: "📂", name: "folder open file directory" },
  { ch: "🗑️", name: "trash delete bin wastebasket" },
  { ch: "➡️", name: "arrow right" },
  { ch: "⬅️", name: "arrow left" },
  { ch: "⬆️", name: "arrow up" },
  { ch: "⬇️", name: "arrow down" },
  { ch: "↩️", name: "arrow return enter back" },
  { ch: "🔄", name: "refresh reload cycle sync" },
  { ch: "➕", name: "plus add math" },
  { ch: "➖", name: "minus subtract math" },
  { ch: "✔️", name: "check mark done yes" },
  { ch: "✖️", name: "multiply cross math" },
  { ch: "©", name: "copyright c" },
  { ch: "®", name: "registered r" },
  { ch: "™", name: "trademark tm" },
  { ch: "→", name: "arrow right" },
  { ch: "←", name: "arrow left" },
  { ch: "↑", name: "arrow up" },
  { ch: "↓", name: "arrow down" },
  { ch: "⇧", name: "shift arrow up" },
  { ch: "⌘", name: "command cmd meta" },
  { ch: "⌥", name: "option alt" },
  { ch: "⏎", name: "return enter" },
  { ch: "⌫", name: "backspace delete" },
  { ch: "°", name: "degree" },
  { ch: "µ", name: "micro mu" },
  { ch: "§", name: "section" },
  { ch: "¶", name: "paragraph pilcrow" },
  { ch: "•", name: "bullet dot" },
  { ch: "–", name: "en dash" },
  { ch: "—", name: "em dash" },
  { ch: "«", name: "quote left guillemet" },
  { ch: "»", name: "quote right guillemet" },
];

let emojiItems = [];
let emojiIndex = 0;

function renderEmoji(query) {
  const grid = $("#emoji-grid");
  grid.innerHTML = "";
  const q = (query || "").trim().toLowerCase();
  const list = q
    ? EMOJI.filter((e) => e.name.includes(q) || e.ch === q)
    : EMOJI;
  emojiItems = list;
  emojiIndex = 0;
  if (list.length === 0) {
    grid.innerHTML = '<div class="emoji-empty">No emoji match "' + escapeHtml(query) + '"</div>';
    return;
  }
  list.forEach((e, i) => {
    const cell = document.createElement("div");
    cell.className = "emoji-cell";
    cell.textContent = e.ch;
    cell.title = e.name;
    cell.addEventListener("click", () => copyRaw(e.ch));
    cell.addEventListener("mouseenter", () => {
      emojiIndex = i;
      highlightEmoji();
    });
    grid.appendChild(cell);
  });
  highlightEmoji();
}

function highlightEmoji(scroll = false) {
  const cells = document.querySelectorAll("#emoji-grid .emoji-cell");
  cells.forEach((c, i) => c.classList.toggle("selected", i === emojiIndex));
  if (scroll && cells[emojiIndex]) cells[emojiIndex].scrollIntoView({ block: "nearest" });
}

async function copyRaw(text) {
  // Use the navigator clipboard for emoji (frontend has user gesture).
  // On failure keep the overlay open and say so — hiding it would look like
  // the copy succeeded.
  try {
    await navigator.clipboard.writeText(text);
  } catch (_) {
    showToast("Couldn't copy — the clipboard isn't writable from the overlay.");
    return;
  }
  await invoke("hide_window");
}

// ===== System tools =====
let depReport = null;
let depBannerDismissed = false;
let toastTimer = null;

function showToast(msg) {
  const t = $("#toast");
  t.textContent = msg;
  t.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.add("hidden"), 3000);
}

function copyFailMsg() {
  const missing = (depReport ? depReport.tools : []).filter(
    (t) => t.group.includes("write") && !t.covered_by
  );
  return missing.length
    ? `Couldn't copy — install ${missing.map((t) => t.name).join(" or ")}.`
    : "Couldn't copy — clipboard write failed.";
}

async function checkDependencies() {
  try {
    depReport = await invoke("check_dependencies");
  } catch (_) {
    depReport = null;
  }
  renderDepBanner();
  renderDepList();
}

// Tools serving a core feature that nothing installed covers → drives the banner.
function missingRequired() {
  const byFeature = {};
  (depReport ? depReport.tools : []).forEach((t) => {
    if (t.required && !t.covered_by) (byFeature[t.feature] = byFeature[t.feature] || []).push(t);
  });
  return byFeature;
}

function renderDepBanner() {
  const banner = $("#dep-banner");
  const parts = Object.entries(missingRequired()).map(
    ([feature, tools]) => `${tools.map((t) => t.name).join(" or ")} — ${tools[0].breaks}`
  );
  if (depBannerDismissed || parts.length === 0) {
    banner.classList.add("hidden");
    return;
  }
  $("#dep-banner-text").textContent =
    `Missing system tools: ${parts.join("  ·  ")}. See Settings → System tools.`;
  banner.classList.remove("hidden");
}

function renderDepList() {
  const list = $("#deps-list");
  const summary = $("#deps-summary");
  if (!depReport) {
    summary.textContent = "Check unavailable.";
    list.innerHTML = "";
    return;
  }
  const missing = depReport.tools.filter((t) => !t.present).length;
  summary.textContent = missing
    ? `${missing} missing on ${depReport.backend}.`
    : `All present (${depReport.backend}).`;
  list.innerHTML = "";
  depReport.tools.forEach((t) => {
    const row = document.createElement("div");
    row.className = "setting-row dep-row";
    const badgeCls = t.present ? "present" : t.covered_by ? "missing covered" : "missing";
    const badge = t.present ? "present" : t.covered_by ? `missing — covered by ${t.covered_by}` : "missing";
    const install = t.present
      ? ""
      : `<div class="dep-install">` +
        ["apt", "dnf", "pacman", "zypper"]
          .map((pm) => `<span>${pm}: ${escapeHtml(t.install[pm])}</span>`)
          .join("") +
        `</div>`;
    const desc =
      escapeHtml(t.feature) +
      (t.present ? "" : ` — ${escapeHtml(t.breaks)}`) +
      (t.note ? ` · ${escapeHtml(t.note)}` : "");
    row.innerHTML = `
      <div>
        <p class="setting-label dep-name">${escapeHtml(t.name)} <span class="dep-badge ${badgeCls}">${badge}</span></p>
        <p class="setting-desc">${desc}</p>
        ${install}
      </div>`;
    list.appendChild(row);
  });
}

$("#dep-banner").addEventListener("click", () => switchView("settings"));
$("#dep-banner-close").addEventListener("click", (e) => {
  e.stopPropagation();
  depBannerDismissed = true;
  $("#dep-banner").classList.add("hidden");
});
$("#deps-recheck").addEventListener("click", checkDependencies);

// ===== Settings UI =====
async function loadSettings() {
  settings = await invoke("get_settings");
  document.body.dataset.theme = settings.theme;
  $("#auto-paste").checked = settings.auto_paste;
  $("#theme").value = settings.theme;
  $("#max-history").value = settings.max_history;
  syncPasteLabel();

  const currentDir = await invoke("get_data_dir");
  $("#data-dir").value = currentDir;

  const pasteTools = (depReport ? depReport.tools : []).filter((t) => t.group === "paste");
  if (pasteTools.length && pasteTools.every((t) => !t.covered_by)) {
    const names = pasteTools.map((t) => t.name).join(" or ");
    $("#paste-desc").textContent = `Install ${names} to enable auto-paste; otherwise press Ctrl+V.`;
  }
  await refreshHotkeyStatus();
}

async function saveSettings() {
  settings = await invoke("set_settings", { settings });
  document.body.dataset.theme = settings.theme;
  // set_settings doesn't emit settings-updated — sync side effects here too.
  syncPasteLabel();
}

$("#auto-paste").addEventListener("change", (e) => { settings.auto_paste = e.target.checked; saveSettings(); });
$("#theme").addEventListener("change", (e) => { settings.theme = e.target.value; saveSettings(); });
$("#max-history").addEventListener("change", (e) => {
  settings.max_history = Math.max(10, Number(e.target.value) || 100);
  saveSettings();
});
$("#data-dir-move").addEventListener("click", async () => {
  const btn = $("#data-dir-move");
  const input = $("#data-dir");
  $("#data-dir-desc").textContent = "Where history and images are stored.";
  btn.disabled = true;
  btn.textContent = "Moving…";
  try {
    const newDir = input.value.trim();
    if (!newDir) {
      $("#data-dir-desc").textContent = "Please enter a directory path.";
      return;
    }
    settings = await invoke("set_data_dir", { newDir });
    input.value = settings.data_dir || newDir;
    $("#data-dir-desc").textContent = "Data moved successfully.";
  } catch (e) {
    $("#data-dir-desc").textContent = "Error: " + e;
  } finally {
    btn.disabled = false;
    btn.textContent = "Move";
  }
});
$("#clear-all").addEventListener("click", async () => {
  items = await invoke("clear_history", { keepPinned: true });
  applyFilter();
});

const HOTKEY_MECH_LABELS = { gnome: "GNOME", kde: "KDE Plasma", cinnamon: "Cinnamon", xfce: "XFCE" };

async function refreshHotkeyStatus() {
  const st = await invoke("hotkey_status");
  const btn = $("#hotkey-toggle");
  const pre = $("#hotkey-instructions");
  if (st.mechanism === "manual") {
    $("#hotkey-status").textContent = "No automatic registration on this desktop — bind it manually:";
    btn.textContent = "Register";
    btn.dataset.reg = "0";
    btn.disabled = true;
  } else {
    const mech = HOTKEY_MECH_LABELS[st.mechanism] || st.mechanism;
    $("#hotkey-status").textContent = st.registered
      ? `Registered via ${mech} — press Ctrl+Shift+V anywhere.`
      : `Not registered (${mech}).`;
    btn.textContent = st.registered ? "Unregister" : "Register";
    btn.dataset.reg = st.registered ? "1" : "0";
    btn.disabled = false;
  }
  // Manual steps: needed on manual desktops, or as a fallback when an
  // automatic mechanism exists but hasn't registered yet. Once registered
  // they hide — nothing left to do.
  if (st.mechanism === "manual" || !st.registered) {
    pre.textContent = st.instructions;
    pre.classList.remove("hidden");
  } else {
    pre.classList.add("hidden");
  }
}

$("#hotkey-toggle").addEventListener("click", async () => {
  const btn = $("#hotkey-toggle");
  btn.disabled = true;
  let error = null;
  try {
    if (btn.dataset.reg === "1") {
      await invoke("unregister_hotkey");
    } else {
      await invoke("register_hotkey", { binding: "<Control><Shift>v" });
    }
  } catch (e) {
    error = e;
  }
  await refreshHotkeyStatus();
  // Write the failure after the refresh — refreshHotkeyStatus rewrites
  // #hotkey-status and would erase the concrete error.
  if (error) $("#hotkey-status").textContent = "Error: " + error;
});

// ===== Search =====
searchEl.addEventListener("input", () => {
  $("#clear-search").classList.toggle("hidden", !searchEl.value);
  if (currentView === "emoji") renderEmoji(searchEl.value);
  else applyFilter();
});
$("#clear-search").addEventListener("click", () => {
  searchEl.value = "";
  $("#clear-search").classList.add("hidden");
  applyFilter();
  searchEl.focus();
});

// ===== Keyboard navigation =====
window.addEventListener("keydown", (e) => {
  if (!detailEl.classList.contains("hidden")) {
    if (e.key === "Escape") detailEl.classList.add("hidden");
    return;
  }
  if (e.key === "Escape") {
    e.preventDefault();
    invoke("hide_window");
    return;
  }
  if (currentView === "emoji") {
    // Column count comes from the live grid style, not a second constant —
    // the CSS owns the layout.
    const cols = emojiCols();
    switch (e.key) {
      case "ArrowRight": emojiIndex = Math.min(emojiIndex + 1, emojiItems.length - 1); break;
      case "ArrowLeft": emojiIndex = Math.max(emojiIndex - 1, 0); break;
      case "ArrowDown": emojiIndex = Math.min(emojiIndex + cols, emojiItems.length - 1); break;
      case "ArrowUp": emojiIndex = Math.max(emojiIndex - cols, 0); break;
      case "Enter":
        e.preventDefault();
        if (emojiItems[emojiIndex]) copyRaw(emojiItems[emojiIndex].ch);
        return;
      case "Tab": cycleView(e); return;
      default: return;
    }
    e.preventDefault();
    highlightEmoji(true);
    return;
  }
  if (currentView !== "history") {
    if (e.key === "Tab") cycleView(e);
    return;
  }
  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1);
      highlightSelected(true);
      break;
    case "ArrowUp":
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      highlightSelected(true);
      break;
    case "Enter":
      e.preventDefault();
      pasteSelected();
      break;
    case "Delete":
      // While typing in the search field, Delete edits text — don't let it
      // delete the selected clip. (Arrows/Enter still drive the list.)
      if (e.target instanceof HTMLInputElement) break;
      e.preventDefault();
      if (filtered[selectedIndex]) deleteItem(filtered[selectedIndex].id);
      break;
    case "Tab":
      cycleView(e);
      break;
    default:
      if ((e.key === "p" || e.key === "P") && (e.ctrlKey || e.altKey)) {
        e.preventDefault();
        if (filtered[selectedIndex]) pinItem(filtered[selectedIndex].id);
      }
  }
});

function emojiCols() {
  // computed gridTemplateColumns is a space-separated list of resolved track
  // sizes (e.g. "58px 58px 58px …"); counting entries gives the column count.
  const tpl = getComputedStyle($("#emoji-grid")).gridTemplateColumns;
  const n = tpl && tpl !== "none" ? tpl.split(" ").filter(Boolean).length : 0;
  return Math.max(1, n || 8);
}

function cycleView(e) {
  e.preventDefault();
  const order = ["history", "emoji", "settings"];
  let idx = order.indexOf(currentView);
  idx = e.shiftKey ? (idx + order.length - 1) % order.length : (idx + 1) % order.length;
  switchView(order[idx]);
}

// ===== Live updates from backend =====
listen("history-updated", (event) => {
  items = event.payload;
  applyFilter();
});
listen("overlay-shown", () => {
  // Reset any stale detail modal from the previous showing.
  detailEl.classList.add("hidden");
  detailItem = null;
  searchEl.value = "";
  $("#clear-search").classList.add("hidden");
  selectedIndex = 0;
  activeFilter = "all";
  document.querySelectorAll(".filter-chip").forEach((c) =>
    c.classList.toggle("active", c.dataset.filter === "all")
  );
  switchView("history");
  refresh();
  searchEl.focus();
  replayEnter();
});

// CSS animations only play once per element — the webview persists across
// hide/show, so replay the panel's enter animation by removing the class,
// forcing a style flush, and re-adding it.
function replayEnter() {
  overlayEl.classList.remove("enter");
  void overlayEl.offsetWidth;
  overlayEl.classList.add("enter");
}
listen("settings-updated", (event) => {
  settings = event.payload;
  document.body.dataset.theme = settings.theme;
  $("#auto-paste").checked = settings.auto_paste;
  $("#theme").value = settings.theme;
  $("#max-history").value = settings.max_history;
  $("#data-dir").value = settings.data_dir || "";
  syncPasteLabel();
});

// The detail dialog's action button copies always and pastes only when
// auto-paste is on — the label has to follow the setting.
function syncPasteLabel() {
  $("#detail-copy").textContent = settings.auto_paste ? "Copy & paste" : "Copy";
}

// ===== Init =====
(async function init() {
  await checkDependencies();
  await loadSettings();
  await refresh();
  searchEl.focus();
})();
