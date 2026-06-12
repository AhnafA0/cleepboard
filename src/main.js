const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ===== State =====
let items = [];
let filtered = [];
let selectedIndex = 0;
let currentView = "history";
let settings = { auto_paste: true, theme: "system", max_history: 100 };

const $ = (sel) => document.querySelector(sel);
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
  const t = item.text || "";
  if (/^https?:\/\//.test(t.trim())) {
    return `<svg viewBox="0 0 24 24"><path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1"/><path d="M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1"/></svg>`;
  }
  if (looksLikeCode(t)) {
    return `<svg viewBox="0 0 24 24"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>`;
  }
  return "T";
}

// ===== Rendering =====
function applyFilter() {
  const q = searchEl.value.trim().toLowerCase();
  filtered = q
    ? items.filter((i) => (i.preview || "").toLowerCase().includes(q))
    : items.slice();
  if (selectedIndex >= filtered.length) selectedIndex = Math.max(0, filtered.length - 1);
  renderList();
}

function renderList() {
  listEl.innerHTML = "";
  if (filtered.length === 0) {
    emptyEl.classList.remove("hidden");
    listEl.classList.add("hidden");
    return;
  }
  emptyEl.classList.add("hidden");
  listEl.classList.remove("hidden");

  const pinned = filtered.filter((i) => i.pinned);
  const rest = filtered.filter((i) => !i.pinned);

  if (pinned.length) {
    listEl.appendChild(sectionHeader("Pinned", pinned.length));
    pinned.forEach((i) => listEl.appendChild(clipEl(i)));
  }
  if (rest.length) {
    const today = rest.filter((i) => isToday(i.timestamp));
    const earlier = rest.filter((i) => !isToday(i.timestamp));
    if (today.length) {
      listEl.appendChild(sectionHeader("Today", today.length));
      today.forEach((i) => listEl.appendChild(clipEl(i)));
    }
    if (earlier.length) {
      listEl.appendChild(sectionHeader("Earlier", earlier.length));
      earlier.forEach((i) => listEl.appendChild(clipEl(i)));
    }
  }
  highlightSelected();
}

function sectionHeader(label, count) {
  const el = document.createElement("div");
  el.className = "section-header";
  el.innerHTML = `<span>${label}</span><span class="count">· ${count}</span>`;
  return el;
}

function clipEl(item) {
  const idx = filtered.indexOf(item);
  const el = document.createElement("div");
  el.className = "clip";
  el.dataset.index = idx;
  el.dataset.id = item.id;

  const icon = iconFor(item);
  const isImg = item.kind === "image";
  const textCls = looksLikeCode(item.text || "") ? "clip-text mono" : "clip-text";
  const pin = item.pinned
    ? `<span class="pin-chip"><svg viewBox="0 0 24 24"><path d="M12 2l2 7h7l-5.5 4 2 7L12 16l-5.5 4 2-7L3 9h7z"/></svg>Pinned</span>`
    : "";

  el.innerHTML = `
    <div class="clip-icon">${icon}</div>
    <div class="clip-body">
      <div class="${textCls}">${escapeHtml(item.preview || "")}</div>
      <div class="clip-meta">${timeAgo(item.timestamp)} ${pin}</div>
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
    highlightSelected();
  });
  return el;
}

async function loadThumb(container, id) {
  const url = await invoke("get_image_data_url", { id });
  if (url) container.innerHTML = `<img src="${url}" alt="" />`;
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

function highlightSelected() {
  document.querySelectorAll(".clip").forEach((el) => {
    const on = Number(el.dataset.index) === selectedIndex;
    el.classList.toggle("selected", on);
    if (on) el.scrollIntoView({ block: "nearest" });
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
  await invoke("copy_item", { id: item.id, paste: settings.auto_paste });
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
  $("#detail-title").textContent = item.kind === "image" ? "Image clip" : "Text clip";
  if (item.kind === "image") {
    const url = await invoke("get_image_data_url", { id: item.id });
    body.innerHTML = `<img src="${url}" alt="" />`;
  } else {
    const full = await invoke("get_item_text", { id: item.id });
    body.innerHTML = `<pre>${escapeHtml(full || item.preview || "")}</pre>`;
  }
  detailEl.classList.remove("hidden");
}

$("#detail-close").addEventListener("click", () => detailEl.classList.add("hidden"));
$("#detail-copy").addEventListener("click", async () => {
  if (detailItem) await invoke("copy_item", { id: detailItem.id, paste: settings.auto_paste });
  detailEl.classList.add("hidden");
});
detailEl.addEventListener("click", (e) => {
  if (e.target === detailEl) detailEl.classList.add("hidden");
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

// ===== Emoji picker =====
const EMOJI = "😀 😂 😍 🥰 😎 🤔 😅 😭 😡 👍 👎 👏 🙏 💪 🔥 ✨ 🎉 ❤️ 💔 ⭐ ✅ ❌ ⚠️ 💡 📌 📎 ✂️ 📋 🔍 🔒 🔓 🚀 🌟 🐛 ⚡ 💻 📱 ⌨️ 🖱️ 📁 📂 🗑️ ➡️ ⬅️ ⬆️ ⬇️ ↩️ 🔄 ➕ ➖ ✔️ ✖️ © ® ™ → ← ↑ ↓ ⇧ ⌘ ⌥ ⏎ ⌫ ° µ § ¶ • – — « »".split(/\s+/).filter(Boolean);

function renderEmoji(query) {
  const grid = $("#emoji-grid");
  grid.innerHTML = "";
  const list = EMOJI; // simple set; search filters by raw char
  list.forEach((ch) => {
    const cell = document.createElement("div");
    cell.className = "emoji-cell";
    cell.textContent = ch;
    cell.addEventListener("click", () => copyRaw(ch));
    grid.appendChild(cell);
  });
}

async function copyRaw(text) {
  // Use the navigator clipboard for emoji (frontend has user gesture).
  try {
    await navigator.clipboard.writeText(text);
  } catch (_) {}
  await invoke("hide_window");
}

// ===== Settings UI =====
async function loadSettings() {
  settings = await invoke("get_settings");
  document.body.dataset.theme = settings.theme;
  $("#auto-paste").checked = settings.auto_paste;
  $("#theme").value = settings.theme;
  $("#max-history").value = settings.max_history;

  const pasteOk = await invoke("paste_tool_available");
  if (!pasteOk) {
    $("#paste-desc").textContent = "Install 'wtype' to enable auto-paste; otherwise press Ctrl+V.";
  }
  await refreshHotkeyStatus();
}

async function saveSettings() {
  settings = await invoke("set_settings", { settings });
  document.body.dataset.theme = settings.theme;
}

$("#auto-paste").addEventListener("change", (e) => { settings.auto_paste = e.target.checked; saveSettings(); });
$("#theme").addEventListener("change", (e) => { settings.theme = e.target.value; saveSettings(); });
$("#max-history").addEventListener("change", (e) => {
  settings.max_history = Math.max(10, Number(e.target.value) || 100);
  saveSettings();
});
$("#clear-all").addEventListener("click", async () => {
  items = await invoke("clear_history", { keepPinned: true });
  applyFilter();
});

async function refreshHotkeyStatus() {
  const reg = await invoke("hotkey_status");
  $("#hotkey-status").textContent = reg ? "Registered ✓ — press Ctrl+Shift+V anywhere." : "Not registered yet.";
  $("#hotkey-toggle").textContent = reg ? "Unregister" : "Register";
  $("#hotkey-toggle").dataset.reg = reg ? "1" : "0";
}

$("#hotkey-toggle").addEventListener("click", async () => {
  const btn = $("#hotkey-toggle");
  btn.disabled = true;
  try {
    if (btn.dataset.reg === "1") {
      await invoke("unregister_hotkey");
    } else {
      await invoke("register_hotkey", { binding: "<Control><Shift>v" });
    }
  } catch (e) {
    $("#hotkey-status").textContent = "Error: " + e;
  }
  btn.disabled = false;
  await refreshHotkeyStatus();
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
  if (currentView !== "history") {
    if (e.key === "Tab") cycleView(e);
    return;
  }
  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1);
      highlightSelected();
      break;
    case "ArrowUp":
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      highlightSelected();
      break;
    case "Enter":
      e.preventDefault();
      pasteSelected();
      break;
    case "Delete":
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
  searchEl.value = "";
  $("#clear-search").classList.add("hidden");
  selectedIndex = 0;
  switchView("history");
  refresh();
  searchEl.focus();
});

// ===== Init =====
(async function init() {
  await loadSettings();
  await refresh();
  searchEl.focus();
})();
