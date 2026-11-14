import { mountSecurityPage } from "./pages/SecurityPage.js";
import { invoke, isDesktopRuntime } from "./services/tauri.js";
import {
  clearSession,
  createNotification,
  getSession,
  listNotifications,
  loginAccount,
  markNotificationRead,
  registerAccount,
  restoreSession,
} from "./services/account.js";

const state = {
  filter: "inbox",
  quickFilter: "all",
  query: "",
  messages: [],
  page: null,
  stats: null,
  settings: {},
  selected: new Set(),
  currentMessage: null,
  composeDraftId: null,
  session: null,
  diagnostics: null,
};

const QUICK_FILTERS = ["all", "unread", "starred", "attachments"];
const FILTER_LABELS = {
  inbox: ["Inbox", "Focused mail from every connected account"],
  starred: ["Starred", "Messages you want close at hand"],
  snoozed: ["Snoozed", "Quiet for now, back when you need them"],
  sent: ["Sent", "Every message delivered from Elyra"],
  drafts: ["Drafts", "Unfinished thoughts, kept locally"],
  archive: ["Archive", "Out of the inbox, still searchable"],
  spam: ["Spam", "Filtered out by the privacy engine"],
  trash: ["Trash", "Review before deleting forever"],
  important: ["Important", "People and decisions that matter"],
  updates: ["Updates", "Product and service activity"],
  security: ["Security", "Sign-ins, privacy and account alerts"],
  changes: ["Changes", "What shipped and what comes next"],
  profile: ["Profile", "Your identity and connected accounts"],
  settings: ["Settings", "Control how Elyra works on this device"],
};

const $ = (selector, scope = document) => scope.querySelector(selector);
const $$ = (selector, scope = document) => [...scope.querySelectorAll(selector)];

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "'": "&#39;",
    '"': "&quot;",
  }[character]));
}

function icon(name, extra = "") {
  return `<svg class="icon ${extra}" aria-hidden="true"><use href="#i-${name}"></use></svg>`;
}

function showToast(message, tone = "default") {
  const stack = $("#toast-stack") || document.body;
  const toast = document.createElement("div");
  toast.className = `toast ${tone}`;
  toast.innerHTML = `${icon(tone === "success" ? "check" : tone === "error" ? "alert" : "info")}<span>${escapeHtml(message)}</span>`;
  stack.append(toast);
  requestAnimationFrame(() => toast.classList.add("visible"));
  window.setTimeout(() => {
    toast.classList.remove("visible");
    window.setTimeout(() => toast.remove(), 220);
  }, 2800);
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value.toFixed(value >= 10 || index === 0 ? 0 : 1)} ${units[index]}`;
}

function formatSync(value) {
  if (!value) return "not synced yet";
  const parsed = new Date(`${String(value).replace(" ", "T")}Z`);
  if (Number.isNaN(parsed.getTime())) return "recently";
  const minutes = Math.max(0, Math.floor((Date.now() - parsed.getTime()) / 60_000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return parsed.toLocaleDateString([], { month: "short", day: "numeric" });
}

function debounce(callback, delay = 180) {
  let timer;
  return (...args) => {
    window.clearTimeout(timer);
    timer = window.setTimeout(() => callback(...args), delay);
  };
}

function setLoading(loading) {
  const list = $("#message-list");
  if (!list) return;
  if (loading) list.setAttribute("aria-busy", "true");
  else list.removeAttribute("aria-busy");
}

function updatePreviewModeBanner() {
  const badge = $(".secure-badge");
  const privacyCard = $(".privacy-card p");
  const preview = !isDesktopRuntime();
  if (badge) {
    badge.innerHTML = `${icon(preview ? "globe" : "shield")} ${preview ? "Browser preview" : "Rust core online"}`;
    badge.title = preview
      ? "Browser preview uses a local fallback. The desktop build runs the Rust mailbox engine."
      : "Mailbox, security and crypto commands are running in Rust.";
  }
  if (privacyCard) {
    privacyCard.textContent = preview
      ? "Preview mode keeps a fallback workspace in this browser. The desktop app uses SQLite and Rust."
      : "Your messages stay in a local SQLite database protected by the Rust core.";
  }
}

function applyPreferences(settings) {
  const theme = settings.theme || "light";
  const accent = settings.accent || "moss";
  const density = settings.density || "comfortable";
  document.body.dataset.theme = theme;
  document.body.dataset.accent = accent;
  document.body.dataset.density = density;
  document.body.classList.toggle("reduce-motion", String(settings.reduce_motion) === "true");
  const themeSelect = $("#theme-select");
  const accentSelect = $("#accent-select");
  const densitySelect = $("#density-select");
  if (themeSelect) themeSelect.value = theme;
  if (accentSelect) accentSelect.value = accent;
  if (densitySelect) densitySelect.value = density;
}

function activeNav(filter) {
  $$("[data-filter]").forEach((item) => item.classList.toggle("active", item.dataset.filter === filter));
}

function showPanel(name) {
  state.filter = name;
  state.selected.clear();
  activeNav(name);
  $("#bulk-bar").hidden = true;
  $("#message-viewer").hidden = true;
  $("#message-list").hidden = true;
  $$("[data-panel]").forEach((panel) => {
    panel.hidden = panel.dataset.panel !== name;
  });
  $("#view-title").textContent = FILTER_LABELS[name]?.[0] || "Elyra";
  $("#view-subtitle").textContent = FILTER_LABELS[name]?.[1] || "";
  $("#content-actions").hidden = true;
  if (name === "changes") renderChanges();
  if (name === "profile") renderProfile();
  if (name === "settings") renderSettings();
}

function showMailbox(filter) {
  state.filter = filter;
  state.query = "";
  state.quickFilter = "all";
  const search = $("#search-input");
  if (search) search.value = "";
  const filterButton = $("#filter-button");
  if (filterButton) filterButton.innerHTML = `All mail ${icon("chevron-down")}`;
  state.selected.clear();
  activeNav(filter);
  $$("[data-panel]").forEach((panel) => { panel.hidden = true; });
  $("#bulk-bar").hidden = true;
  $("#message-viewer").hidden = true;
  $("#message-list").hidden = false;
  $("#content-actions").hidden = false;
  updateHeader();
  loadMailbox();
}

function updateHeader() {
  const [title] = FILTER_LABELS[state.filter] || ["Inbox"];
  const page = state.page;
  $("#view-title").textContent = title;
  const subtitle = $("#view-subtitle");
  if (!page) {
    subtitle.textContent = "Loading local mailbox…";
  } else if (state.query) {
    subtitle.textContent = `${page.total} result${page.total === 1 ? "" : "s"} for “${state.query}”`;
  } else {
    subtitle.textContent = `${page.total} message${page.total === 1 ? "" : "s"} · ${page.unread} unread · ${formatSync(state.stats?.last_sync_at)}`;
  }
  const selectAll = $("#select-all");
  if (selectAll) {
    selectAll.checked = state.messages.length > 0 && state.messages.every((message) => state.selected.has(message.id));
    selectAll.indeterminate = state.selected.size > 0 && !selectAll.checked;
  }
  renderBulkBar();
}

function renderBulkBar() {
  const bar = $("#bulk-bar");
  if (!bar) return;
  const count = state.selected.size;
  bar.hidden = count === 0;
  const countNode = $("#bulk-count");
  if (countNode) countNode.textContent = `${count} selected`;
  $$("[data-bulk]").forEach((button) => {
    const action = button.dataset.bulk;
    if (state.filter === "trash") {
      button.hidden = !["restore", "delete"].includes(action);
    } else {
      button.hidden = ["restore"].includes(action);
    }
  });
}

function renderMessages() {
  const list = $("#message-list");
  if (!list) return;
  if (!state.messages.length) {
    list.innerHTML = `<div class="empty-state">${icon("search", "empty-icon")}<h3>No messages here</h3><p>Try another mailbox, clear the search, or change the quick filter.</p><button class="ghost-button" id="empty-reset" type="button">${icon("refresh")} Reset view</button></div>`;
    $("#empty-reset")?.addEventListener("click", () => {
      state.quickFilter = "all";
      state.query = "";
      $("#search-input").value = "";
      $("#filter-button").innerHTML = `All mail ${icon("chevron-down")}`;
      loadMailbox();
    });
    updateHeader();
    return;
  }

  list.innerHTML = state.messages.map((message) => `
    <article class="message-row ${message.is_read ? "" : "unread"} ${state.selected.has(message.id) ? "selected" : ""}" data-message-id="${message.id}">
      <label class="select-box" aria-label="Select ${escapeHtml(message.subject)}">
        <input type="checkbox" data-select="${message.id}" ${state.selected.has(message.id) ? "checked" : ""} />
      </label>
      <button class="star-btn ${message.is_starred ? "selected" : ""}" type="button" data-star="${message.id}" aria-label="${message.is_starred ? "Unstar" : "Star"} ${escapeHtml(message.subject)}">
        ${icon("star")}
      </button>
      <span class="sender-avatar ${message.color}">${escapeHtml(message.initials)}</span>
      <div class="message-copy">
        <div class="message-line">
          <b>${escapeHtml(message.sender)}</b>
          <span class="message-tag ${message.color}">${escapeHtml(message.tag)}</span>
          ${message.has_attachments ? `<span class="attachment-hint" title="Has attachments">${icon("paperclip")}</span>` : ""}
          <span class="message-time">${escapeHtml(message.time)}</span>
        </div>
        <p><strong>${escapeHtml(message.subject)}</strong><span>${escapeHtml(message.preview)}</span></p>
      </div>
      <span class="row-arrow">${icon("chevron-right")}</span>
    </article>
  `).join("");
  updateHeader();
}

async function loadMailbox({ keepSelection = false } = {}) {
  setLoading(true);
  try {
    const page = await invoke("list_mailbox", {
      filter: state.filter,
      quickFilter: state.quickFilter,
      query: state.query,
      limit: 200,
      offset: 0,
    });
    state.page = page;
    state.messages = page.messages;
    if (!keepSelection) state.selected.clear();
    renderMessages();
  } catch (error) {
    console.error("Failed to load mailbox", error);
    showToast("Could not load the local mailbox", "error");
  } finally {
    setLoading(false);
  }
}

function updateStats(stats) {
  state.stats = stats;
  const folderCounts = stats.folder_counts || {};
  const folderUnread = stats.folder_unread || {};
  $$("[data-count]").forEach((node) => {
    const key = node.dataset.count;
    const count = key === "inbox"
      ? stats.unread
      : key === "starred"
        ? stats.starred
        : key === "drafts"
          ? stats.drafts
          : key === "spam"
            ? stats.spam
            : folderUnread[key] ?? folderCounts[key] ?? 0;
    node.textContent = key === "inbox" ? String(stats.unread) : String(count);
    node.hidden = Number(count) === 0;
  });

  const set = (selector, value) => {
    const node = $(selector);
    if (node) node.textContent = value;
  };
  set("#stat-total", String(stats.total));
  set("#stat-unread", String(stats.unread));
  set("#stat-starred", String(stats.starred));
  set("#stat-attachments", String(stats.attachments));
  set("#stat-storage", formatBytes(stats.storage_bytes));
  set("#stat-accounts", String(stats.accounts ?? 3));
  set("#storage-label", `${formatBytes(stats.storage_bytes)} local`);
  const meter = $("#storage-meter");
  if (meter) meter.style.setProperty("--value", `${Math.min(100, Math.max(4, (stats.storage_bytes / (50 * 1024 * 1024)) * 100))}%`);
  set("#sync-status", `Synced ${formatSync(stats.last_sync_at)}`);
  const profileTotal = $("#profile-total");
  const profileUnread = $("#profile-unread");
  const profileStarred = $("#profile-starred");
  if (profileTotal) profileTotal.textContent = String(stats.total);
  if (profileUnread) profileUnread.textContent = String(stats.unread);
  if (profileStarred) profileStarred.textContent = String(stats.starred);
}

async function refreshStats() {
  try {
    updateStats(await invoke("mailbox_stats"));
  } catch (error) {
    console.error("Failed to load mailbox statistics", error);
  }
}

async function refreshMailbox(button) {
  if (button) {
    button.disabled = true;
    button.classList.add("spinning");
  }
  try {
    const stats = await invoke("refresh_mailbox");
    updateStats(stats);
    await loadMailbox({ keepSelection: true });
    showToast("Mailbox is up to date", "success");
  } catch (error) {
    console.error("Refresh failed", error);
    showToast("Refresh failed", "error");
  } finally {
    if (button) {
      button.disabled = false;
      button.classList.remove("spinning");
    }
  }
}

function renderChanges() {
  const list = $("#changes-list");
  if (!list) return;
  invoke("get_changelog").then((entries) => {
    list.innerHTML = entries.map((entry) => `
      <article class="change-card ${entry.status}">
        <div class="change-meta">
          <span class="version-pill">v${escapeHtml(entry.version)}</span>
          <span>${escapeHtml(entry.date)}</span>
        </div>
        <h3>${escapeHtml(entry.title)}</h3>
        <p>${escapeHtml(entry.detail)}</p>
        <span class="change-status">${entry.status === "next" ? icon("clock") + " Planned" : icon("check") + " Shipped"}</span>
      </article>
    `).join("");
  }).catch((error) => {
    console.error("Failed to load changelog", error);
    list.innerHTML = `<div class="empty-state"><h3>Changes unavailable</h3><p>The Rust changelog command did not respond.</p></div>`;
  });
}

function renderProfile() {
  if (!state.stats) return;
  const name = state.session?.user?.displayName || state.settings.display_name || "Alex Smith";
  const email = state.session?.user?.email || state.settings.domain_handle || "alex@elyra.com";
  const initials = String(name).split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase() || "AS";
  const nodes = {
    "#profile-name": name,
    "#profile-email": email,
    "#profile-initials": initials,
    "#profile-total": String(state.stats.total),
    "#profile-unread": String(state.stats.unread),
    "#profile-starred": String(state.stats.starred),
    "#profile-accounts": String(state.stats.accounts ?? 3),
  };
  Object.entries(nodes).forEach(([selector, value]) => {
    const node = $(selector);
    if (node) node.textContent = value;
  });
  const sessionState = $("#profile-session-state");
  if (sessionState) {
    sessionState.innerHTML = state.session
      ? `${icon("cloud")} Signed in to Elyra Cloud`
      : `${icon("lock")} Local-only workspace`;
  }
}

async function renderDiagnostics() {
  const grid = $("#diagnostics-grid");
  if (!grid) return;
  grid.innerHTML = `<div class="diag-line">Loading Rust diagnostics…</div>`;
  try {
    const info = await invoke("get_app_diagnostics");
    state.diagnostics = info;
    grid.innerHTML = `
      <div><small>App version</small><strong>${escapeHtml(info.app_version)}</strong></div>
      <div><small>Engine</small><strong>${escapeHtml(info.engine)}</strong></div>
      <div><small>Platform</small><strong>${escapeHtml(info.os)} / ${escapeHtml(info.arch)}</strong></div>
      <div><small>SQLite</small><strong>${escapeHtml(info.sqlite_version)}</strong></div>
      <div><small>Database</small><strong title="${escapeHtml(info.database_path)}">${formatBytes(info.database_bytes)}</strong></div>
      <div><small>Cloud API</small><strong>${info.cloud_configured ? "Configured" : "Local default"}</strong></div>
      <div class="diag-path"><small>Data location</small><code>${escapeHtml(info.database_path)}</code></div>
    `;
  } catch (error) {
    console.error("Diagnostics failed", error);
    grid.innerHTML = `<div class="diag-line error">Diagnostics unavailable.</div>`;
  }
}

function renderSettings() {
  const settings = state.settings;
  const bind = (selector, value) => {
    const node = $(selector);
    if (node) node.value = value ?? "";
  };
  bind("#display-name", settings.display_name || "Alex Smith");
  bind("#domain-handle", settings.domain_handle || "alex@elyra.com");
  bind("#signature", settings.signature || "");
  bind("#theme-select", settings.theme || "light");
  bind("#accent-select", settings.accent || "moss");
  bind("#density-select", settings.density || "comfortable");
  const toggle = (selector, value) => {
    const node = $(selector);
    if (node) node.checked = String(value) === "true";
  };
  toggle("#local-only", settings.local_only ?? "true");
  toggle("#desktop-notifications", settings.desktop_notifications ?? "true");
  toggle("#reduce-motion", settings.reduce_motion ?? "false");
  const authState = $("#settings-auth-state");
  if (authState) {
    authState.innerHTML = state.session
      ? `<span class="status-dot online"></span> Signed in as ${escapeHtml(state.session.user.displayName)}`
      : `<span class="status-dot"></span> Local-only — connect Elyra Cloud for notifications`;
  }
  renderDiagnostics();
}

function updateAccountUi() {
  const session = state.session;
  const name = session?.user?.displayName || state.settings.display_name || "Alex Smith";
  const email = session?.user?.email || state.settings.domain_handle || "alex@elyra.com";
  const initials = String(name).split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase() || "AS";
  const accountButton = $(".account-button");
  if (accountButton) {
    accountButton.innerHTML = `<span class="avatar small">${escapeHtml(initials)}</span><span class="account-copy"><b>${escapeHtml(name)}</b><small>${escapeHtml(email)}</small></span><span class="account-chevron">${icon("chevron-down")}</span>`;
  }
  $$(".avatar").forEach((node) => {
    if (!node.classList.contains("small")) node.textContent = initials;
  });
  const topName = $("#welcome-name");
  if (topName) topName.textContent = name.split(/\s+/)[0] || "there";
  $("#domain-preview") && ($("#domain-preview").textContent = state.settings.domain_handle || "alex@elyra.com");
  renderProfile();
}

async function refreshNotifications() {
  if (!state.session) {
    const badge = $(".notification-count");
    if (badge) badge.hidden = true;
    return [];
  }
  try {
    const notifications = await listNotifications();
    const unread = notifications.filter((item) => !item.is_read).length;
    const badge = $(".notification-count");
    if (badge) {
      badge.textContent = String(unread);
      badge.hidden = unread === 0;
    }
    return notifications;
  } catch (error) {
    console.error("Notification refresh failed", error);
    return [];
  }
}

async function openNotifications() {
  const popover = $("#notification-popover");
  if (!state.session) {
    showAuthModal("login");
    return;
  }
  try {
    const notifications = await refreshNotifications();
    popover.innerHTML = `
      <div class="popover-header">
        <div><span class="eyebrow">Elyra Cloud</span><h3>Notifications</h3></div>
        <button class="icon-button" id="close-notifications" type="button" aria-label="Close">${icon("x")}</button>
      </div>
      ${notifications.length ? notifications.map((item) => `
        <button class="notification-item ${item.is_read ? "" : "unread"}" data-notification-id="${escapeHtml(item.id)}" type="button">
          <b>${escapeHtml(item.title)}</b>
          <span>${escapeHtml(item.body)}</span>
          <small>${new Date(item.created_at).toLocaleString()}</small>
        </button>
      `).join("") : `<div class="empty-state compact"><h3>All quiet</h3><p>No notifications yet.</p></div>`}
    `;
    popover.hidden = false;
    $("#close-notifications")?.addEventListener("click", () => { popover.hidden = true; });
    $$("[data-notification-id]", popover).forEach((item) => item.addEventListener("click", async () => {
      await markNotificationRead(item.dataset.notificationId).catch((error) => showToast(error.message, "error"));
      item.classList.remove("unread");
      await refreshNotifications();
    }));
  } catch (error) {
    showToast(error.message || "Could not load notifications", "error");
  }
}

function showAuthModal(mode = "login") {
  const gate = $("#auth-gate");
  gate.hidden = false;
  gate.innerHTML = `
    <form class="modal-card auth-modal" id="auth-form">
      <div class="modal-header">
        <div><span class="eyebrow">Elyra account</span><h2>${mode === "login" ? "Welcome back" : "Create your account"}</h2></div>
        <button class="icon-button" id="close-auth" type="button" aria-label="Close">${icon("x")}</button>
      </div>
      ${mode === "register" ? `<label>Display name<input name="displayName" required maxlength="120" placeholder="Alex Smith" /></label>` : ""}
      <label>Email<input name="email" type="email" autocomplete="email" required placeholder="you@example.com" /></label>
      <label>Password<input name="password" type="password" minlength="12" autocomplete="${mode === "login" ? "current-password" : "new-password"}" required placeholder="At least 12 characters" /></label>
      <p class="form-error" id="auth-error" role="alert"></p>
      <div class="modal-footer">
        <button class="text-button" id="auth-switch" type="button">${mode === "login" ? "Create an account" : "I already have an account"}</button>
        <span class="modal-hint">${isDesktopRuntime() ? "Rust cloud bridge" : "Browser preview"}</span>
        <button class="primary-button" type="submit">${mode === "login" ? "Sign in" : "Register"} ${icon("arrow-right")}</button>
      </div>
    </form>
  `;
  $("#close-auth").addEventListener("click", () => { gate.hidden = true; });
  $("#auth-switch").addEventListener("click", () => showAuthModal(mode === "login" ? "register" : "login"));
  $("#auth-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const submit = form.querySelector("[type=submit]");
    const errorNode = form.querySelector("#auth-error");
    submit.disabled = true;
    errorNode.textContent = "";
    try {
      await (mode === "login"
        ? loginAccount({ email: data.get("email"), password: data.get("password") })
        : registerAccount({ email: data.get("email"), password: data.get("password"), displayName: data.get("displayName") }));
      state.session = getSession();
      gate.hidden = true;
      updateAccountUi();
      renderSettings();
      await refreshNotifications();
      showToast(mode === "login" ? "Signed in" : "Account created", "success");
    } catch (error) {
      errorNode.textContent = error.message;
      submit.disabled = false;
    }
  });
}

function openNotificationComposer() {
  if (!state.session) {
    showAuthModal("login");
    return;
  }
  const modal = document.createElement("div");
  modal.className = "modal-backdrop";
  modal.innerHTML = `
    <form class="modal-card" id="notification-form">
      <div class="modal-header">
        <div><span class="eyebrow">Private delivery</span><h2>Send notification</h2></div>
        <button class="icon-button" data-close type="button" aria-label="Close">${icon("x")}</button>
      </div>
      <label>Recipient email<input name="recipient" type="email" required placeholder="friend@example.com" /></label>
      <label>Title<input name="title" required maxlength="160" placeholder="A quick update" /></label>
      <label class="body-field">Message<textarea name="body" required maxlength="4000" rows="6" placeholder="Write a notification..."></textarea></label>
      <p class="form-error" data-error role="alert"></p>
      <div class="modal-footer">
        <span class="modal-hint">Recipient must have an Elyra account.</span>
        <button class="primary-button" type="submit">Send ${icon("arrow-right")}</button>
      </div>
    </form>
  `;
  document.body.append(modal);
  modal.querySelector("[data-close]").addEventListener("click", () => modal.remove());
  modal.addEventListener("click", (event) => { if (event.target === modal) modal.remove(); });
  modal.querySelector("form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const submit = form.querySelector("[type=submit]");
    submit.disabled = true;
    try {
      await createNotification({ recipientEmail: data.get("recipient"), title: data.get("title"), body: data.get("body") });
      modal.remove();
      showToast("Notification delivered", "success");
    } catch (error) {
      form.querySelector("[data-error]").textContent = error.message;
      submit.disabled = false;
    }
  });
}

function openComposer(initial = {}) {
  let modal = $("#compose-modal");
  state.composeDraftId = initial.draftId || null;
  if (!modal) {
    modal = document.createElement("div");
    modal.id = "compose-modal";
    modal.className = "modal-backdrop";
    modal.innerHTML = `
      <form class="modal-card compose-modal" id="compose-form">
        <div class="modal-header">
          <div><span class="eyebrow">New message</span><h2>${initial.draftId ? "Edit draft" : "Write with intention"}</h2></div>
          <button class="icon-button" id="close-compose" type="button" aria-label="Close">${icon("x")}</button>
        </div>
        <label>To<input name="to" type="email" autocomplete="email" placeholder="name@example.com" /></label>
        <label>Subject<input name="subject" placeholder="A clear subject" /></label>
        <label class="body-field">Message<textarea name="body" rows="10" placeholder="Write your message..."></textarea></label>
        <div class="modal-footer">
          <button class="text-button" id="save-draft" type="button">${icon("draft")} Save draft</button>
          <button class="text-button danger-text" id="delete-draft" type="button" hidden>${icon("trash")} Delete draft</button>
          <span class="modal-hint">${isDesktopRuntime() ? "Saved by the Rust core" : "Saved in this browser preview"}</span>
          <button class="primary-button" type="submit">Send ${icon("send")}</button>
        </div>
      </form>
    `;
    document.body.append(modal);
    modal.addEventListener("click", (event) => {
      if (event.target === modal || event.target.closest("#close-compose")) modal.remove();
    });
    modal.querySelector("#save-draft").addEventListener("click", async () => {
      const form = modal.querySelector("#compose-form");
      const data = new FormData(form);
      try {
        await invoke("save_draft", {
          id: state.composeDraftId,
          recipient: data.get("to") || "",
          subject: data.get("subject") || "(no subject)",
          body: data.get("body") || "",
        });
        modal.remove();
        state.composeDraftId = null;
        await refreshStats();
        await loadMailbox();
        showToast("Draft saved locally", "success");
      } catch (error) {
        showToast(error.message || "Could not save draft", "error");
      }
    });
    modal.querySelector("#delete-draft").addEventListener("click", async () => {
      if (!state.composeDraftId) return;
      await invoke("delete_draft", { id: state.composeDraftId }).catch(() => {});
      modal.remove();
      state.composeDraftId = null;
      await refreshStats();
      await loadMailbox();
      showToast("Draft deleted");
    });
    modal.querySelector("#compose-form").addEventListener("submit", async (event) => {
      event.preventDefault();
      const form = event.currentTarget;
      const data = new FormData(form);
      const submit = form.querySelector("[type=submit]");
      if (!data.get("to") || !data.get("subject")) {
        showToast("Add a recipient and a subject first", "error");
        return;
      }
      submit.disabled = true;
      submit.innerHTML = `${icon("refresh")} Sending…`;
      try {
        await invoke("send_email", { to: data.get("to"), subject: data.get("subject"), body: data.get("body") });
        if (state.composeDraftId) await invoke("delete_draft", { id: state.composeDraftId }).catch(() => {});
        modal.remove();
        state.composeDraftId = null;
        await refreshStats();
        await loadMailbox();
        showToast("Message sent", "success");
      } catch (error) {
        submit.disabled = false;
        submit.innerHTML = `Send ${icon("send")}`;
        showToast(error.message || "Could not send. Check SMTP settings.", "error");
      }
    });
  }
  const form = modal.querySelector("#compose-form");
  form.elements.to.value = initial.to || "";
  form.elements.subject.value = initial.subject || "";
  form.elements.body.value = initial.body || state.settings.signature || "";
  modal.querySelector("h2").textContent = initial.draftId ? "Edit draft" : "Write with intention";
  modal.querySelector("#delete-draft").hidden = !initial.draftId;
  modal.hidden = false;
  form.elements.to.focus();
}

async function openMessage(id) {
  try {
    const detail = await invoke("get_message", { id });
    state.currentMessage = detail;
    const message = detail.message;
    const viewer = $("#message-viewer");
    $("#bulk-bar").hidden = true;
    $("#message-list").hidden = true;
    $("#content-actions").hidden = true;
    viewer.hidden = false;
    const replyTo = message.folder.toLowerCase() === "sent"
      ? detail.recipient
      : `${message.sender.toLowerCase().replace(/[^a-z0-9]+/g, ".")}@example.com`;
    const isDraft = message.folder.toLowerCase() === "drafts";
    viewer.innerHTML = `
      <div class="viewer-toolbar">
        <button class="back-button" id="close-viewer" type="button">${icon("arrow-left")} Back to ${escapeHtml(FILTER_LABELS[state.filter]?.[0] || state.filter)}</button>
        <div class="viewer-tools">
          <button class="icon-button" data-viewer-action="archive" title="Archive" type="button">${icon("archive")}</button>
          <button class="icon-button" data-viewer-action="spam" title="Move to spam" type="button">${icon("spam")}</button>
          <button class="icon-button" data-viewer-action="trash" title="Move to trash" type="button">${icon("trash")}</button>
          <button class="icon-button" data-viewer-action="more" title="More" type="button">${icon("more")}</button>
        </div>
      </div>
      <div class="viewer-heading">
        <span class="sender-avatar ${message.color}">${escapeHtml(message.initials)}</span>
        <div class="viewer-title">
          <div class="viewer-labels"><span class="message-tag ${message.color}">${escapeHtml(message.tag)}</span><span class="secure-inline">${icon("lock")} local</span></div>
          <h2>${escapeHtml(message.subject)}</h2>
          <p><b>${escapeHtml(message.sender)}</b> &lt;${escapeHtml(message.folder.toLowerCase() === "sent" ? detail.recipient : replyTo)}&gt;</p>
        </div>
        <span class="message-time">${escapeHtml(message.time)}</span>
      </div>
      <div class="viewer-actions">
        <button type="button" id="viewer-star" class="${message.is_starred ? "selected" : ""}">${icon("star")} ${message.is_starred ? "Starred" : "Star"}</button>
        ${isDraft ? `<button type="button" id="viewer-edit">${icon("edit")} Edit draft</button>` : `<button type="button" id="viewer-reply">${icon("reply")} Reply</button><button type="button" id="viewer-forward">${icon("forward")} Forward</button>`}
        <button type="button" id="viewer-unread">${icon("mail")} Mark unread</button>
      </div>
      <div class="viewer-body">${escapeHtml(detail.body_text).replace(/\n/g, "<br />")}</div>
      ${detail.attachments.length ? `<div class="attachment-list">${detail.attachments.map((attachment) => `
        <div class="attachment-card">
          ${icon("paperclip", "attachment-icon")}
          <span><b>${escapeHtml(attachment.name)}</b><small>${escapeHtml(attachment.mime_type)} · ${formatBytes(attachment.size_bytes)}</small></span>
          <button type="button" data-download="${escapeHtml(attachment.name)}" title="Download attachment">${icon("download")}</button>
        </div>
      `).join("")}</div>` : ""}
    `;
    $("#close-viewer").addEventListener("click", () => {
      viewer.hidden = true;
      $("#message-list").hidden = false;
      $("#content-actions").hidden = false;
      refreshStats();
      loadMailbox({ keepSelection: true });
    });
    $("#viewer-star").addEventListener("click", async (event) => {
      const next = !message.is_starred;
      await invoke("update_message", { id: message.id, isStarred: next }).catch(() => {});
      event.currentTarget.classList.toggle("selected", next);
      event.currentTarget.innerHTML = `${icon("star")} ${next ? "Starred" : "Star"}`;
      message.is_starred = next;
      await refreshStats();
      await loadMailbox({ keepSelection: true });
    });
    $("#viewer-unread")?.addEventListener("click", async () => {
      await invoke("update_message", { id: message.id, isRead: false }).catch(() => {});
      viewer.hidden = true;
      $("#message-list").hidden = false;
      $("#content-actions").hidden = false;
      await loadMailbox();
      showToast("Marked unread");
    });
    $("#viewer-edit")?.addEventListener("click", () => {
      openComposer({ to: detail.recipient, subject: message.subject, body: detail.body_text, draftId: message.id });
    });
    $("#viewer-reply")?.addEventListener("click", () => {
      openComposer({
        to: replyTo,
        subject: `Re: ${message.subject}`,
        body: `\n\n---\nOn ${message.received_at}, ${message.sender} wrote:\n> ${detail.body_text.split("\n").join("\n> ")}`,
      });
    });
    $("#viewer-forward")?.addEventListener("click", () => {
      openComposer({
        subject: `Fwd: ${message.subject}`,
        body: `\n\n--- Forwarded message ---\nFrom: ${message.sender}\nSubject: ${message.subject}\n\n${detail.body_text}`,
      });
    });
    $$("[data-viewer-action]", viewer).forEach((button) => button.addEventListener("click", async () => {
      const action = button.dataset.viewerAction;
      if (action === "more") {
        showToast("More actions are available from the command palette");
        return;
      }
      await invoke("bulk_update_messages", { ids: [message.id], action }).catch((error) => showToast(error.message, "error"));
      viewer.hidden = true;
      $("#message-list").hidden = false;
      $("#content-actions").hidden = false;
      await refreshStats();
      await loadMailbox();
      showToast(`Message moved to ${action}`);
    }));
    $$("[data-download]", viewer).forEach((button) => button.addEventListener("click", () => {
      showToast(`Attachment “${button.dataset.download}” is stored in the local preview`);
    }));
    updateStats((await invoke("mailbox_stats")));
  } catch (error) {
    console.error("Failed to open message", error);
    showToast("Could not open this message", "error");
  }
}

async function bulkAction(action) {
  const ids = [...state.selected];
  if (!ids.length) return;
  if (action === "delete" && !window.confirm(`Permanently delete ${ids.length} message${ids.length === 1 ? "" : "s"}?`)) return;
  try {
    await invoke("bulk_update_messages", { ids, action });
    state.selected.clear();
    await refreshStats();
    await loadMailbox();
    showToast(`${ids.length} message${ids.length === 1 ? "" : "s"} updated`, "success");
  } catch (error) {
    showToast(error.message || "Bulk action failed", "error");
  }
}

function openCommandPalette() {
  const palette = $("#command-palette");
  const input = $("#command-input");
  const list = $("#command-list");
  const actions = [
    { label: "Compose a message", hint: "New email", icon: "compose", run: () => openComposer() },
    { label: "Refresh mailbox", hint: "Sync now", icon: "refresh", run: () => refreshMailbox($("#refresh-button")) },
    { label: "Show unread mail", hint: "Filter", icon: "mail", run: () => { state.quickFilter = "unread"; $("#filter-button").innerHTML = `Unread ${icon("chevron-down")}`; loadMailbox(); } },
    { label: "Show starred mail", hint: "Filter", icon: "star", run: () => { state.quickFilter = "starred"; $("#filter-button").innerHTML = `Starred ${icon("chevron-down")}`; loadMailbox(); } },
    { label: "Mark this mailbox read", hint: "Bulk action", icon: "check", run: async () => { await invoke("mark_all_read", { filter: state.filter }); await refreshStats(); await loadMailbox(); showToast("Mailbox marked read", "success"); } },
    { label: "Open notifications", hint: "Elyra Cloud", icon: "bell", run: openNotifications },
    { label: "Open settings", hint: "Preferences", icon: "settings", run: () => showPanel("settings") },
    { label: "Open changes", hint: "Release notes", icon: "changes", run: () => showPanel("changes") },
    { label: "Export mailbox", hint: "JSON backup", icon: "download", run: exportMailbox },
    { label: "Toggle theme", hint: "Light / dark", icon: "palette", run: () => {
      const themes = ["light", "dark", "midnight", "oled"];
      const next = themes[(themes.indexOf(document.body.dataset.theme) + 1) % themes.length];
      state.settings.theme = next;
      applyPreferences(state.settings);
      invoke("set_local_settings", { values: { theme: next } }).catch(() => {});
      showToast(`Theme: ${next}`);
    } },
  ];
  const render = () => {
    const query = input.value.trim().toLowerCase();
    const visible = actions.filter((action) => `${action.label} ${action.hint}`.toLowerCase().includes(query));
    list.innerHTML = visible.length ? visible.map((action, index) => `
      <button class="command-item ${index === 0 ? "selected" : ""}" type="button">
        ${icon(action.icon)}
        <span>${escapeHtml(action.label)}</span>
        <small>${escapeHtml(action.hint)}</small>
      </button>
    `).join("") : `<div class="command-empty">No matching action</div>`;
    $$(".command-item", list).forEach((item, index) => item.addEventListener("click", () => {
      palette.hidden = true;
      visible[index].run();
    }));
  };
  palette.hidden = false;
  input.value = "";
  render();
  input.focus();
  input.oninput = render;
  input.onkeydown = (event) => {
    const items = $$(".command-item", list);
    const current = items.findIndex((item) => item.classList.contains("selected"));
    if (event.key === "Escape") palette.hidden = true;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!items.length) return;
      const next = event.key === "ArrowDown" ? (current + 1) % items.length : (current - 1 + items.length) % items.length;
      items.forEach((item, index) => item.classList.toggle("selected", index === next));
    }
    if (event.key === "Enter" && current >= 0) items[current].click();
  };
}

async function exportMailbox() {
  try {
    const payload = await invoke("export_mailbox");
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `elyra-mailbox-${new Date().toISOString().slice(0, 10)}.json`;
    document.body.append(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
    showToast("Mailbox exported", "success");
  } catch (error) {
    showToast(error.message || "Export failed", "error");
  }
}

function bindEvents() {
  $("#search-input").addEventListener("input", debounce((event) => {
    state.query = event.target.value;
    loadMailbox();
  }, 180));

  $("#filter-button").addEventListener("click", () => {
    const next = QUICK_FILTERS[(QUICK_FILTERS.indexOf(state.quickFilter) + 1) % QUICK_FILTERS.length];
    state.quickFilter = next;
    const label = next === "all" ? "All mail" : `${next[0].toUpperCase()}${next.slice(1)}`;
    $("#filter-button").innerHTML = `${label} ${icon("chevron-down")}`;
    loadMailbox();
  });

  $("#compose-button").addEventListener("click", () => openComposer());
  $("#refresh-button").addEventListener("click", (event) => refreshMailbox(event.currentTarget));
  $(".notification-button").addEventListener("click", openNotifications);
  $("#notification-button")?.addEventListener("click", openNotificationComposer);
  $("#connect-account")?.addEventListener("click", () => showAuthModal("login"));
  $("#add-account")?.addEventListener("click", () => showAuthModal("register"));
  $("#manage-account")?.addEventListener("click", () => showPanel("profile"));
  $("#settings-button").addEventListener("click", () => showPanel("settings"));
  $("#profile-settings")?.addEventListener("click", () => showPanel("settings"));
  $("#domain-copy")?.addEventListener("click", async () => {
    const value = state.settings.domain_handle || "alex@elyra.com";
    await navigator.clipboard?.writeText(value).catch(() => {});
    showToast("Elyra address copied", "success");
  });

  $$("[data-filter]").forEach((item) => item.addEventListener("click", () => {
    const filter = item.dataset.filter;
    if (["changes", "profile", "settings"].includes(filter)) showPanel(filter);
    else showMailbox(filter);
  }));

  $("#settings-panel").addEventListener("click", (event) => {
    if (event.target.closest("#save-settings")) saveSettingsFromForm();
    if (event.target.closest("#reset-demo")) resetDemo();
    if (event.target.closest("#export-mailbox")) exportMailbox();
    if (event.target.closest("#refresh-diagnostics")) renderDiagnostics();
    if (event.target.closest("#sign-out")) signOut();
    if (event.target.closest("#open-auth")) showAuthModal("login");
  });

  $("#settings-form")?.addEventListener("submit", (event) => {
    event.preventDefault();
    saveSettingsFromForm();
  });

  $("#theme-select")?.addEventListener("change", (event) => {
    persistSettings({ theme: event.target.value });
  });
  $("#accent-select")?.addEventListener("change", (event) => {
    persistSettings({ accent: event.target.value });
  });
  $("#density-select")?.addEventListener("change", (event) => {
    persistSettings({ density: event.target.value });
  });
  $("#reduce-motion")?.addEventListener("change", (event) => {
    persistSettings({ reduce_motion: event.target.checked });
  });

  $("#message-list").addEventListener("click", (event) => {
    const star = event.target.closest("[data-star]");
    if (star) {
      event.stopPropagation();
      const id = Number(star.dataset.star);
      const message = state.messages.find((item) => item.id === id);
      if (!message) return;
      const next = !message.is_starred;
      message.is_starred = next;
      star.classList.toggle("selected", next);
      invoke("update_message", { id, isStarred: next }).catch(() => {});
      refreshStats();
      return;
    }
    const row = event.target.closest(".message-row");
    if (row && !event.target.closest("input, .select-box, .star-btn")) openMessage(Number(row.dataset.messageId));
  });

  $("#message-list").addEventListener("change", (event) => {
    const checkbox = event.target.closest("[data-select]");
    if (!checkbox) return;
    const id = Number(checkbox.dataset.select);
    if (checkbox.checked) state.selected.add(id);
    else state.selected.delete(id);
    checkbox.closest(".message-row")?.classList.toggle("selected", checkbox.checked);
    updateHeader();
  });

  $("#select-all")?.addEventListener("change", (event) => {
    state.selected.clear();
    if (event.target.checked) state.messages.forEach((message) => state.selected.add(message.id));
    renderMessages();
  });

  $("#bulk-bar")?.addEventListener("click", (event) => {
    const button = event.target.closest("[data-bulk]");
    if (button) bulkAction(button.dataset.bulk);
  });

  $("#command-palette").addEventListener("click", (event) => {
    if (event.target.id === "command-palette") event.currentTarget.hidden = true;
  });

  document.addEventListener("keydown", (event) => {
    const typing = ["INPUT", "TEXTAREA", "SELECT"].includes(document.activeElement?.tagName);
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      openCommandPalette();
      return;
    }
    if (event.key === "Escape") {
      $("#compose-modal")?.remove();
      $("#command-palette").hidden = true;
      $("#notification-popover").hidden = true;
      $("#auth-gate").hidden = true;
      return;
    }
    if (typing) return;
    if (event.key === "/") {
      event.preventDefault();
      $("#search-input").focus();
    }
    if (event.key.toLowerCase() === "c") {
      event.preventDefault();
      openComposer();
    }
    if (event.key.toLowerCase() === "r") {
      event.preventDefault();
      refreshMailbox($("#refresh-button"));
    }
    if (event.key.toLowerCase() === "u") {
      state.quickFilter = "unread";
      $("#filter-button").innerHTML = `Unread ${icon("chevron-down")}`;
      loadMailbox();
    }
  });

  document.addEventListener("click", (event) => {
    if (!event.target.closest("#notification-popover") && !event.target.closest(".notification-button")) {
      $("#notification-popover").hidden = true;
    }
  });
}

async function persistSettings(patch = {}) {
  const values = { ...state.settings, ...patch };
  state.settings = values;
  applyPreferences(values);
  try {
    await invoke("set_local_settings", { values });
  } catch (error) {
    showToast(error.message || "Could not save preference", "error");
  }
}

async function saveSettingsFromForm() {
  const values = {
    display_name: $("#display-name").value.trim() || "Alex Smith",
    domain_handle: $("#domain-handle").value.trim() || "alex@elyra.com",
    theme: $("#theme-select").value,
    accent: $("#accent-select").value,
    density: $("#density-select").value,
    signature: $("#signature").value,
    local_only: $("#local-only").checked,
    desktop_notifications: $("#desktop-notifications").checked,
    reduce_motion: $("#reduce-motion").checked,
  };
  try {
    await invoke("set_local_settings", { values });
    state.settings = { ...state.settings, ...values };
    applyPreferences(state.settings);
    updateAccountUi();
    $("#save-settings").innerHTML = `${icon("check")} Saved locally`;
    window.setTimeout(() => { $("#save-settings").innerHTML = `${icon("check")} Save changes`; }, 1600);
    showToast("Settings saved", "success");
  } catch (error) {
    showToast(error.message || "Could not save settings", "error");
  }
}

async function resetDemo() {
  if (!window.confirm("Reset the local demo mailbox? Your current local test messages will be replaced.")) return;
  try {
    const stats = await invoke("reset_demo_mailbox");
    updateStats(stats);
    await loadMailbox();
    showToast("Demo mailbox restored", "success");
  } catch (error) {
    showToast(error.message || "Could not reset demo data", "error");
  }
}

async function signOut() {
  await clearSession();
  state.session = null;
  updateAccountUi();
  renderSettings();
  showToast("Signed out of Elyra Cloud");
}

async function boot() {
  state.session = await restoreSession();
  try {
    state.settings = await invoke("get_local_settings");
  } catch {
    state.settings = {};
  }
  applyPreferences(state.settings);
  updatePreviewModeBanner();
  updateAccountUi();
  bindEvents();
  mountSecurityPage($("#settings-panel"));

  try {
    await invoke("seed_demo_mailbox");
  } catch (error) {
    console.error("Demo seed failed", error);
  }
  await refreshStats();
  await loadMailbox();
  renderChanges();
  renderProfile();
  renderSettings();
  renderDiagnostics();
  refreshNotifications().catch(() => {});
  window.setInterval(() => refreshNotifications().catch(() => {}), 30000);
}

document.addEventListener("DOMContentLoaded", boot);
