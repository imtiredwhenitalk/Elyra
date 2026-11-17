import { mountSecurityPage } from "./pages/SecurityPage.js";
import { analyzeMessage, getSenderReputation, listSecurityAlerts, reportFalsePositive, reportReputationIssue } from "./services/security.js";
import { invoke, isDesktopRuntime } from "./services/tauri.js";
import {
  clearSession,
  createNotification,
  getApiBase,
  getSession,
  listNotifications,
  loginAccount,
  markNotificationRead,
  registerAccount,
  restoreSession,
  setApiBase,
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
  accounts: [],
  presets: [],
  editingAccountId: null,
  apiBase: "",
  cloudMailRelay: false,
  threats: new Map(),
  senderRules: [],
};

const QUICK_FILTERS = ["all", "unread", "starred", "important", "high", "urgent", "attachments"];
const QUICK_FILTER_LABELS = {
  all: "All mail",
  unread: "Unread",
  starred: "Starred",
  important: "Important",
  high: "High priority",
  urgent: "Urgent",
  attachments: "Attachments",
};
const PRIORITY_ORDER = ["low", "normal", "high", "urgent"];
const PRIORITY_LABELS = { low: "Low", normal: "Normal", high: "High", urgent: "Urgent" };
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

function priorityValue(value) {
  const normalized = String(value || "").toLowerCase();
  return PRIORITY_ORDER.includes(normalized) ? normalized : "normal";
}

function priorityLabel(value) {
  return PRIORITY_LABELS[priorityValue(value)];
}

function prioritySourceLabel(message) {
  if (String(message.priority_source || "auto") === "manual") return "Set manually.";
  return message.priority_reason || "Suggested automatically.";
}

function importanceFlag(message) {
  if (!message.is_important) return "";
  return `<span class="importance-flag" title="Marked important">${icon("important")}</span>`;
}

function priorityBadge(message) {
  const priority = priorityValue(message.priority);
  if (priority === "normal") return "";
  return `<span class="priority-pill ${priority}" title="${escapeHtml(prioritySourceLabel(message))}">${icon("priority")} ${priorityLabel(priority)}</span>`;
}

function blockedRuleLabel(value) {
  if (!value) return "";
  const [scope, ...rest] = String(value).split(":");
  const target = rest.join(":") || value;
  return scope === "domain" ? `Blocked domain ${target}` : `Blocked sender ${target}`;
}

function blockedBadge(message) {
  if (!message.blocked_by) return "";
  return `<span class="blocked-chip" title="${escapeHtml(blockedRuleLabel(message.blocked_by))}">${icon("spam")} Blocked</span>`;
}

function setQuickFilter(value) {
  state.quickFilter = value;
  const button = $("#filter-button");
  if (button) button.innerHTML = `${QUICK_FILTER_LABELS[value] || "All mail"} ${icon("chevron-down")}`;
  loadMailbox();
}

function openExternalUrl(url) {
  const opener = window.__TAURI__?.opener;
  if (opener?.openUrl) {
    opener.openUrl(url).catch(() => {});
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

function threatFlag(id) {
  const threat = state.threats.get(id);
  if (!threat) return "";
  return `<span class="threat-flag ${escapeHtml(threat.risk)}" title="Elyra Shield: ${escapeHtml(threat.risk)} risk">${icon("shield")}</span>`;
}

const REPUTATION_LABELS = {
  trusted: "Trusted",
  neutral: "Neutral",
  suspicious: "Suspicious",
  malicious: "Malicious",
};

function reputationStamp(value) {
  if (!value) return "just now";
  const parsed = new Date(`${String(value).replace(" ", "T")}Z`);
  if (Number.isNaN(parsed.getTime())) return String(value);
  return parsed.toLocaleString([], { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function authBadge(label, value) {
  const normalized = String(value || "").toLowerCase();
  const tone = normalized === "pass"
    ? "pass"
    : ["fail", "softfail", "permerror", "temperror"].includes(normalized)
      ? "fail"
      : normalized
        ? "warn"
        : "missing";
  return `<span class="auth-badge ${tone}" title="${escapeHtml(label)} result">${escapeHtml(label)} <b>${escapeHtml(normalized || "n/a")}</b></span>`;
}

function reputationPanel(reputation) {
  const classification = ["trusted", "neutral", "suspicious", "malicious"].includes(reputation.classification)
    ? reputation.classification
    : "neutral";
  const label = REPUTATION_LABELS[classification];
  const auth = reputation.auth || {};
  const authBadges = [
    authBadge("SPF", auth.spf),
    authBadge("DKIM", auth.dkim),
    authBadge("DMARC", auth.dmarc),
  ].join("");
  const reasons = (reputation.reasons || []).map((reason) => `<li>${escapeHtml(reason)}</li>`).join("");
  return `
    <div class="reputation-card ${classification}">
      <div class="reputation-head">
        <span class="reputation-badge ${classification}">${icon("shield")} ${label}</span>
        <span class="reputation-domain" title="Sender domain">${escapeHtml(reputation.domain || "Unknown domain")}</span>
        <span class="reputation-auth">${authBadges}</span>
        <button class="icon-button tiny" type="button" data-refresh-reputation title="Refresh sender reputation">${icon("refresh")}</button>
      </div>
      ${reasons ? `<ul class="reputation-reasons">${reasons}</ul>` : ""}
      <div class="reputation-foot">
        <small>${reputation.disputed ? "Assessment reported" : `Updated ${escapeHtml(reputationStamp(reputation.updatedAt))}`}</small>
        ${reputation.disputed
          ? `<span class="reputation-reported">${icon("check")} Reported${reputation.feedbackNote ? `: ${escapeHtml(reputation.feedbackNote)}` : ""}</span>`
          : `<button class="text-button" type="button" data-report-reputation>Report incorrect assessment</button>`}
      </div>
    </div>
  `;
}

function bindReputationActions(host, id) {
  host.querySelector("[data-refresh-reputation]")?.addEventListener("click", async (event) => {
    const button = event.currentTarget;
    button.disabled = true;
    try {
      const reputation = await getSenderReputation(id);
      renderReputationCard(host, reputation, id);
      showToast("Sender reputation refreshed", "success");
    } catch (error) {
      button.disabled = false;
      showToast(error.message || "Could not refresh the reputation", "error");
    }
  });
  host.querySelector("[data-report-reputation]")?.addEventListener("click", async (event) => {
    const note = window.prompt("What looks wrong with this sender assessment?", "");
    if (note === null) return;
    const button = event.currentTarget;
    button.disabled = true;
    try {
      const reputation = await reportReputationIssue(id, note);
      renderReputationCard(host, reputation, id);
      showToast("Assessment reported", "success");
    } catch (error) {
      button.disabled = false;
      showToast(error.message || "Could not report the assessment", "error");
    }
  });
}

function renderReputationCard(host, reputation, id) {
  host.innerHTML = reputationPanel(reputation);
  bindReputationActions(host, id);
}

async function renderSenderReputation(viewer, id) {
  const host = viewer.querySelector("#sender-reputation");
  if (!host) return;
  host.innerHTML = `<div class="reputation-card loading">${icon("shield")} Checking sender domain reputation…</div>`;
  try {
    renderReputationCard(host, await getSenderReputation(id), id);
  } catch (error) {
    host.innerHTML = "";
    console.error("Sender reputation failed", error);
  }
}

function threatBanner(security) {
  const risk = security.risk || "safe";
  const reported = Boolean(security.falsePositive);
  const reasons = (security.reasons || []).map((reason) => `<li>${escapeHtml(reason)}</li>`).join("");
  const urls = (security.urls || []).filter((item) => item && item.url);
  const linkRows = urls.length
    ? `<div class="threat-links">${urls.map((item) => {
        const verdict = item.verdict || "unscanned";
        const label = verdict === "unscanned" || verdict === "unknown" ? "not checked" : verdict;
        return `<button type="button" class="threat-link ${escapeHtml(verdict)}" data-threat-link="${escapeHtml(item.url)}" data-verdict="${escapeHtml(verdict)}"><span>${escapeHtml(item.host || item.url)}</span><em>${escapeHtml(label)}</em></button>`;
      }).join("")}</div>`
    : "";
  if (risk === "safe") {
    const note = security.externalScanned
      ? "Sender signals and links checked."
      : "Local signals checked. External link scanning is off.";
    return `<div class="threat-banner safe">${icon("shield")}<div><b>Elyra Shield: no threats detected</b><span>${note}${reported ? " Reported as a false positive." : ""}</span>${linkRows}</div></div>`;
  }
  return `<div class="threat-banner ${escapeHtml(risk)}">${icon("alert")}<div><b>Elyra Shield flagged this message as ${escapeHtml(risk)}</b><span>Risk score ${Number(security.score) || 0}/100</span><ul class="threat-reasons">${reasons}</ul>${linkRows}</div><div class="threat-actions">${reported ? `<span class="threat-reported">Reported</span>` : `<button class="ghost-button" type="button" data-report-false-positive>Report false positive</button>`}</div></div>`;
}

async function renderMessageShield(viewer, id) {
  const host = viewer.querySelector("#message-shield");
  if (!host) return;
  host.innerHTML = `<div class="threat-banner checking">${icon("shield")}<div><b>Elyra Shield is checking this message…</b><span>Local signals first, then your connected threat providers.</span></div></div>`;
  try {
    const security = await analyzeMessage(id, false);
    host.innerHTML = threatBanner(security);
    host.querySelector("[data-report-false-positive]")?.addEventListener("click", async (event) => {
      const button = event.currentTarget;
      button.disabled = true;
      try {
        const updated = await reportFalsePositive(id, "");
        host.innerHTML = threatBanner(updated);
        showToast("Reported as a false positive", "success");
        loadMailbox({ keepSelection: true }).catch(() => {});
      } catch (error) {
        button.disabled = false;
        showToast(error.message || "Could not report the message", "error");
      }
    });
    host.querySelectorAll("[data-threat-link]").forEach((link) => link.addEventListener("click", (event) => {
      event.preventDefault();
      const url = link.dataset.threatLink;
      const verdict = link.dataset.verdict;
      if (verdict === "malicious") {
        showToast("Blocked: this link is flagged as malicious", "error");
        return;
      }
      if ((verdict === "suspicious" || verdict === "unscanned") && !window.confirm("This link has not been verified as safe. Open it anyway?")) return;
      openExternalUrl(url);
    }));
  } catch (error) {
    host.innerHTML = "";
    console.error("Shield analysis failed", error);
  }
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

const PROVIDER_LABELS = {
  gmail: "Gmail",
  outlook: "Outlook",
  icloud: "iCloud",
  yahoo: "Yahoo",
  fastmail: "Fastmail",
  proton: "Proton",
  elyra: "Elyra local",
  custom: "Custom",
};

function providerLabel(provider) {
  return PROVIDER_LABELS[provider] || "Custom";
}

function providerLetter(provider) {
  return providerLabel(provider).slice(0, 1).toUpperCase();
}

function providerClass(provider) {
  return ["gmail", "proton", "elyra"].includes(provider) ? provider : "custom";
}

function accountStatus(account) {
  if (account.syncError) {
    return `<span class="account-status error" title="${escapeHtml(account.syncError)}">Sync error</span>`;
  }
  if (!account.imapHost) return `<span class="account-status muted">Local only</span>`;
  if (!account.hasPassword) return `<span class="account-status warn">Password missing</span>`;
  return `<span class="account-status">${account.lastSyncAt ? `Synced ${formatSync(account.lastSyncAt)}` : "Ready to sync"}</span>`;
}

function accountRow(account) {
  return `
    <div class="connected-account" data-account-id="${account.id}">
      <span class="provider-icon ${providerClass(account.provider)}">${escapeHtml(providerLetter(account.provider))}</span>
      <span class="account-copy">
        <b>${escapeHtml(account.displayName || account.email)}</b>
        <small>${escapeHtml(account.email)}</small>
      </span>
      ${accountStatus(account)}
      <span class="account-actions">
        <button class="icon-button tiny" type="button" data-account-action="sync" data-account-id="${account.id}" title="Sync now" aria-label="Sync ${escapeHtml(account.email)}">${icon("refresh")}</button>
        <button class="icon-button tiny" type="button" data-account-action="edit" data-account-id="${account.id}" title="Edit account" aria-label="Edit ${escapeHtml(account.email)}">${icon("settings")}</button>
        <button class="icon-button tiny danger-text" type="button" data-account-action="delete" data-account-id="${account.id}" title="Remove account" aria-label="Remove ${escapeHtml(account.email)}">${icon("trash")}</button>
      </span>
    </div>
  `;
}

function renderComposeAccounts() {
  const select = $("#compose-account");
  if (!select) return;
  const previous = select.value;
  const sendable = (state.accounts || []).filter((account) => account.smtpHost);
  select.innerHTML = `<option value="">Environment SMTP</option>${sendable
    .map((account) => `<option value="${account.id}">${escapeHtml(account.email)}</option>`)
    .join("")}`;
  if (previous && [...select.options].some((option) => option.value === previous)) {
    select.value = previous;
  } else if (sendable.length) {
    select.value = String(sendable[0].id);
  }
}

function renderAccounts() {
  const accounts = state.accounts || [];
  const detail = $("#connected-accounts");
  if (detail) {
    detail.innerHTML = accounts.length
      ? accounts.map(accountRow).join("")
      : `<p class="detail-note">No mail accounts yet. Add one to sync an IMAP inbox.</p>`;
  }
  const settings = $("#settings-accounts");
  if (settings) {
    settings.innerHTML = accounts.length
      ? accounts.map(accountRow).join("")
      : `<p class="detail-note">Add an IMAP account to pull mail into the local mailbox.</p>`;
  }
  const count = $("#profile-accounts");
  if (count) count.textContent = String(accounts.length);
  const syncAll = $("#sync-all-accounts");
  if (syncAll) syncAll.disabled = !accounts.some((account) => account.imapHost);
  renderComposeAccounts();
}

async function loadAccounts() {
  try {
    state.accounts = await invoke("list_email_accounts");
  } catch (error) {
    console.error("Failed to load email accounts", error);
    state.accounts = [];
  }
  renderAccounts();
  return state.accounts;
}

async function loadProviderPresets() {
  if (state.presets.length) return state.presets;
  try {
    state.presets = await invoke("provider_presets");
  } catch (error) {
    console.error("Failed to load provider presets", error);
    state.presets = [];
  }
  return state.presets;
}

async function syncAccount(accountId, button) {
  if (button) {
    button.disabled = true;
    button.classList.add("spinning");
  }
  try {
    const result = await invoke("sync_email_account", { accountId, limit: 40 });
    await loadAccounts();
    await refreshStats();
    await loadMailbox({ keepSelection: true });
    const label = result.imported === 1 ? "1 new message" : `${result.imported} new messages`;
    showToast(`Synced ${label}`, "success");
  } catch (error) {
    await loadAccounts();
    showToast(error.message || "IMAP sync failed", "error");
  } finally {
    if (button) {
      button.disabled = false;
      button.classList.remove("spinning");
    }
  }
}

async function syncAllAccounts() {
  try {
    return await invoke("sync_all_accounts", { limit: 40 });
  } catch (error) {
    console.error("Sync all failed", error);
    return { accounts: [], imported: 0, failed: true };
  }
}

function accountInputFromForm(form) {
  const data = new FormData(form);
  return {
    id: state.editingAccountId,
    provider: data.get("provider"),
    email: data.get("email"),
    displayName: data.get("displayName"),
    username: data.get("username"),
    password: data.get("password") || null,
    imapHost: data.get("imapHost"),
    imapPort: Number(data.get("imapPort")) || 993,
    imapSecurity: data.get("imapSecurity"),
    smtpHost: data.get("smtpHost"),
    smtpPort: Number(data.get("smtpPort")) || 465,
    smtpSecurity: data.get("smtpSecurity"),
  };
}

function applyPreset(modal, presetId) {
  const preset = state.presets.find((item) => item.id === presetId);
  const form = modal.querySelector("form");
  if (!preset || preset.id === "custom") {
    form.elements.imapHost.value = "";
    form.elements.imapPort.value = "993";
    form.elements.imapSecurity.value = "tls";
    form.elements.smtpHost.value = "";
    form.elements.smtpPort.value = "465";
    form.elements.smtpSecurity.value = "tls";
    const hint = modal.querySelector("[data-account-hint]");
    if (hint) hint.textContent = preset?.hint || "Enter the server settings from your provider.";
    return;
  }
  form.elements.imapHost.value = preset.imapHost;
  form.elements.imapPort.value = String(preset.imapPort);
  form.elements.imapSecurity.value = preset.imapSecurity;
  form.elements.smtpHost.value = preset.smtpHost;
  form.elements.smtpPort.value = String(preset.smtpPort);
  form.elements.smtpSecurity.value = preset.smtpSecurity;
  const hint = modal.querySelector("[data-account-hint]");
  if (hint) hint.textContent = preset.hint;
}

async function openAccountModal(accountId = null) {
  await loadProviderPresets();
  const account = state.accounts.find((item) => Number(item.id) === Number(accountId));
  state.editingAccountId = account?.id || null;
  const providers = [
    ...state.presets.filter((preset) => preset.id !== "custom"),
    { id: "elyra", label: "Elyra local", hint: "A local Elyra workspace account. Add IMAP settings to sync a real inbox." },
    ...state.presets.filter((preset) => preset.id === "custom"),
  ];
  const selectedProvider = account?.provider || "gmail";
  const modal = document.createElement("div");
  modal.className = "modal-backdrop";
  modal.id = "account-modal";
  modal.innerHTML = `
    <form class="modal-card account-modal" id="account-form">
      <div class="modal-header">
        <div><span class="eyebrow">Mail account</span><h2>${account ? "Edit account" : "Connect a mailbox"}</h2></div>
        <button class="icon-button" data-close type="button" aria-label="Close">${icon("x")}</button>
      </div>
      <div class="field-grid">
        <label>Provider
          <select name="provider" id="account-provider">
            ${providers.map((preset) => `<option value="${preset.id}" ${preset.id === selectedProvider ? "selected" : ""}>${escapeHtml(preset.label)}</option>`).join("")}
          </select>
        </label>
        <label>Email address
          <input name="email" type="email" required autocomplete="email" value="${escapeHtml(account?.email || "")}" placeholder="you@example.com" />
        </label>
      </div>
      <div class="field-grid">
        <label>Display name
          <input name="displayName" value="${escapeHtml(account?.displayName || "")}" placeholder="Personal" />
        </label>
        <label>Username
          <input name="username" autocomplete="username" value="${escapeHtml(account?.username || "")}" placeholder="Usually the email address" />
        </label>
      </div>
      <label>Password
        <input name="password" type="password" autocomplete="current-password" placeholder="${account?.hasPassword ? "Stored in the OS keychain — leave blank to keep" : "IMAP / app password"}" />
      </label>
      <p class="account-hint" data-account-hint></p>

      <fieldset class="account-server">
        <legend>Incoming mail (IMAP)</legend>
        <div class="field-grid three">
          <label>Server<input name="imapHost" value="${escapeHtml(account?.imapHost || "")}" placeholder="imap.example.com" /></label>
          <label>Port<input name="imapPort" type="number" min="1" max="65535" value="${account?.imapPort || 993}" /></label>
          <label>Security
            <select name="imapSecurity">
              <option value="tls" ${account?.imapSecurity !== "starttls" ? "selected" : ""}>TLS</option>
              <option value="starttls" ${account?.imapSecurity === "starttls" ? "selected" : ""}>STARTTLS</option>
            </select>
          </label>
        </div>
      </fieldset>

      <fieldset class="account-server">
        <legend>Outgoing mail (SMTP)</legend>
        <div class="field-grid three">
          <label>Server<input name="smtpHost" value="${escapeHtml(account?.smtpHost || "")}" placeholder="smtp.example.com" /></label>
          <label>Port<input name="smtpPort" type="number" min="1" max="65535" value="${account?.smtpPort || 465}" /></label>
          <label>Security
            <select name="smtpSecurity">
              <option value="tls" ${account?.smtpSecurity !== "starttls" ? "selected" : ""}>TLS</option>
              <option value="starttls" ${account?.smtpSecurity === "starttls" ? "selected" : ""}>STARTTLS</option>
            </select>
          </label>
        </div>
      </fieldset>

      <p class="test-result" data-test-result></p>
      <p class="form-error" data-error role="alert"></p>
      <div class="modal-footer">
        ${account ? `<button class="text-button danger-text" type="button" data-delete-account>${icon("trash")} Remove</button>` : ""}
        <button class="ghost-button" type="button" data-test-account>${icon("refresh")} Test connection</button>
        <span class="modal-hint">${isDesktopRuntime() ? "Secrets are stored in the Windows Credential Manager" : "Browser preview stores account settings locally"}</span>
        <button class="primary-button" type="submit">${icon("check")} ${account ? "Save account" : "Connect account"}</button>
      </div>
    </form>
  `;
  document.body.append(modal);
  const form = modal.querySelector("form");
  const hint = () => modal.querySelector("[data-account-hint]");
  const initialPreset = state.presets.find((preset) => preset.id === selectedProvider);
  if (initialPreset) hint().textContent = initialPreset.hint;
  modal.querySelector("#account-provider").addEventListener("change", (event) => applyPreset(modal, event.target.value));
  modal.querySelector("[data-close]").addEventListener("click", () => modal.remove());
  modal.addEventListener("click", (event) => { if (event.target === modal) modal.remove(); });

  modal.querySelector("[data-test-account]").addEventListener("click", async (event) => {
    const button = event.currentTarget;
    const result = modal.querySelector("[data-test-result]");
    const error = modal.querySelector("[data-error]");
    button.disabled = true;
    result.textContent = "Testing IMAP and SMTP…";
    result.className = "test-result";
    error.textContent = "";
    try {
      const outcome = await invoke("test_email_settings", { account: accountInputFromForm(form) });
      result.textContent = `${outcome.imapOk ? "IMAP ready" : "IMAP failed"} · ${outcome.smtpOk ? "SMTP ready" : "SMTP failed"} — ${outcome.detail}`;
      result.classList.add(outcome.imapOk && outcome.smtpOk ? "success" : "error");
    } catch (testError) {
      result.textContent = testError.message || "Connection test failed";
      result.classList.add("error");
    } finally {
      button.disabled = false;
    }
  });

  modal.querySelector("[data-delete-account]")?.addEventListener("click", async () => {
    if (!account) return;
    if (!window.confirm(`Remove ${account.email} and its local messages from Elyra?`)) return;
    try {
      await invoke("delete_email_account", { id: account.id });
      modal.remove();
      await loadAccounts();
      await refreshStats();
      await loadMailbox();
      showToast("Account removed", "success");
    } catch (deleteError) {
      modal.querySelector("[data-error]").textContent = deleteError.message || "Could not remove the account";
    }
  });

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const error = modal.querySelector("[data-error]");
    const submit = form.querySelector("[type=submit]");
    error.textContent = "";
    submit.disabled = true;
    try {
      await invoke("save_email_account", { account: accountInputFromForm(form) });
      modal.remove();
      await loadAccounts();
      await refreshStats();
      showToast(account ? "Account updated" : "Mailbox connected", "success");
    } catch (saveError) {
      error.textContent = saveError.message || "Could not save the account";
      submit.disabled = false;
    }
  });

  form.elements.email.focus();
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
    <article class="message-row ${message.is_read ? "" : "unread"} ${message.is_important ? "important" : ""} ${state.selected.has(message.id) ? "selected" : ""}" data-message-id="${message.id}">
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
          ${importanceFlag(message)}
          ${priorityBadge(message)}
          ${blockedBadge(message)}
          ${threatFlag(message.id)}
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
    try {
      const alerts = await listSecurityAlerts(200);
      state.threats = new Map(alerts.map((alert) => [Number(alert.emailId), alert]));
    } catch {
      state.threats = new Map();
    }
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
        : key === "important"
          ? stats.important
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
  set("#stat-accounts", String(stats.accounts ?? state.accounts.length));
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
    const summary = await syncAllAccounts();
    const failed = (summary.accounts || []).filter((account) => account.error);
    const stats = await invoke("refresh_mailbox");
    updateStats(stats);
    await loadAccounts();
    await loadMailbox({ keepSelection: true });
    if (failed.length) {
      showToast(`${failed.length} account${failed.length === 1 ? "" : "s"} could not sync`, "error");
    } else if (summary.imported > 0) {
      showToast(`Synced ${summary.imported} new message${summary.imported === 1 ? "" : "s"}`, "success");
    } else {
      showToast("Mailbox is up to date", "success");
    }
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
    "#profile-accounts": String(state.accounts.length || state.stats.accounts || 0),
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
      <div><small>OS keychain</small><strong>${info.keychain_available ? "Connected" : "Unavailable"}</strong></div>
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
  toggle("#auto-priority", settings.auto_priority ?? "true");
  bind("#priority-default", settings.priority_default || "normal");
  bind("#blocked-sender-action", settings.blocked_sender_action || "spam");
  toggle("#attachment-scanning", settings.attachment_scanning ?? "true");
  toggle("#attachment-clamav", settings.attachment_clamav ?? "false");
  bind("#clamav-path", settings.clamav_path || "clamscan");
  toggle("#attachment-virustotal", settings.attachment_virustotal ?? "false");
  bind("#attachment-scan-max", settings.attachment_scan_max_mb || "25");
  bind("#api-url-input", state.apiBase || "");
  renderSenderRules();
  loadSenderRules();
  const authState = $("#settings-auth-state");
  if (authState) {
    authState.innerHTML = state.session
      ? `<span class="status-dot online"></span> Signed in as ${escapeHtml(state.session.user.displayName)}${state.cloudMailRelay ? " · mail relay ready" : ""}`
      : `<span class="status-dot"></span> Local-only — connect Elyra Cloud for notifications and server mail relay`;
  }
  renderDiagnostics();
}

function senderRuleScopeLabel(rule) {
  return rule.scope === "domain" ? "Domain" : "Address";
}

function renderSenderRules() {
  const list = $("#sender-rules-list");
  if (!list) return;
  const rules = Array.isArray(state.senderRules) ? state.senderRules : [];
  if (!rules.length) {
    list.innerHTML = `<p class="settings-note">No sender rules yet. Add an address or domain to block it or keep it trusted.</p>`;
    return;
  }
  const groups = [
    { kind: "block", label: "Blocked" },
    { kind: "allow", label: "Trusted" },
  ];
  list.innerHTML = groups.map((group) => {
    const items = rules.filter((rule) => rule.kind === group.kind);
    if (!items.length) return "";
    return `
      <div class="sender-rule-group">
        <h4>${group.label}</h4>
        ${items.map((rule) => `
          <div class="sender-rule-card">
            <div class="sender-rule-copy">
              <b>${escapeHtml(rule.value)}</b>
              <small>${senderRuleScopeLabel(rule)}${rule.note ? ` · ${escapeHtml(rule.note)}` : ""}</small>
            </div>
            <button class="icon-button" type="button" data-delete-sender-rule="${rule.id}" aria-label="Delete rule for ${escapeHtml(rule.value)}">${icon("x")}</button>
          </div>
        `).join("")}
      </div>
    `;
  }).join("");
}

async function loadSenderRules() {
  try {
    const rules = await invoke("list_sender_rules");
    state.senderRules = Array.isArray(rules) ? rules : [];
    renderSenderRules();
  } catch (error) {
    console.error("Could not load sender rules", error);
  }
}

async function addSenderRule() {
  const kind = $("#sender-rule-kind")?.value || "block";
  const scope = $("#sender-rule-scope")?.value || "address";
  const value = $("#sender-rule-value")?.value.trim() || "";
  const note = $("#sender-rule-note")?.value.trim() || "";
  if (!value) {
    showToast("Enter an address or domain", "error");
    return;
  }
  try {
    await invoke("save_sender_rule", { kind, scope, value, note: note || null });
    if ($("#sender-rule-value")) $("#sender-rule-value").value = "";
    if ($("#sender-rule-note")) $("#sender-rule-note").value = "";
    await loadSenderRules();
    await refreshStats();
    await loadMailbox({ keepSelection: true });
    showToast(kind === "block" ? "Sender blocked" : "Sender trusted", "success");
  } catch (error) {
    showToast(error.message || "Could not save the rule", "error");
  }
}

async function deleteSenderRule(id) {
  try {
    await invoke("delete_sender_rule", { id });
    await loadSenderRules();
    await refreshStats();
    await loadMailbox({ keepSelection: true });
    showToast("Rule removed", "success");
  } catch (error) {
    showToast(error.message || "Could not remove the rule", "error");
  }
}

async function applySenderRules({ quiet = false } = {}) {
  try {
    const blocked = await invoke("apply_sender_rules");
    await refreshStats();
    await loadMailbox({ keepSelection: true });
    if (!quiet) {
      showToast(blocked
        ? `Rules applied · ${blocked} message${blocked === 1 ? "" : "s"} blocked`
        : "Sender rules applied", "success");
    }
  } catch (error) {
    showToast(error.message || "Could not apply sender rules", "error");
  }
}

async function setBlockedSenderAction(value) {
  const action = value === "trash" ? "trash" : "spam";
  await persistSettings({ blocked_sender_action: action });
  await applySenderRules({ quiet: true });
  showToast(`Blocked mail now moves to ${action === "trash" ? "Trash" : "Spam"}`, "success");
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

async function refreshCloudMailStatus() {
  if (!state.session || !isDesktopRuntime()) {
    state.cloudMailRelay = false;
    return false;
  }
  try {
    const status = await invoke("cloud_mail_status");
    state.cloudMailRelay = Boolean(status?.configured);
  } catch {
    state.cloudMailRelay = false;
  }
  return state.cloudMailRelay;
}

function showAuthModal(mode = "login", { onboarding = false } = {}) {
  const gate = $("#auth-gate");
  gate.hidden = false;
  gate.innerHTML = `
    <form class="modal-card auth-modal" id="auth-form">
      <div class="modal-header">
        <div><span class="eyebrow">${onboarding ? "Welcome to Elyra" : "Elyra account"}</span><h2>${mode === "login" ? "Welcome back" : "Create your account"}</h2></div>
        <button class="icon-button" id="close-auth" type="button" aria-label="Close">${icon("x")}</button>
      </div>
      ${onboarding ? `<p class="auth-intro">Register once to send mail to any address through your Elyra server, receive notifications, and keep your account in sync. You can also continue offline.</p>` : ""}
      ${mode === "register" ? `<label>Display name<input name="displayName" required maxlength="120" placeholder="Alex Smith" /></label>` : ""}
      <label>Email<input name="email" type="email" autocomplete="email" required placeholder="you@example.com" /></label>
      <label>Password<input name="password" type="password" minlength="12" autocomplete="${mode === "login" ? "current-password" : "new-password"}" required placeholder="At least 12 characters" /></label>
      <label>Server URL<input name="apiUrl" type="url" value="${escapeHtml(state.apiBase && state.apiBase !== "http://127.0.0.1:8787" ? state.apiBase : "")}" placeholder="https://your-server.example/api" /></label>
      <p class="auth-server-hint">Use the address of your Elyra server. Local development defaults to http://127.0.0.1:8787.</p>
      <p class="form-error" id="auth-error" role="alert"></p>
      <div class="modal-footer">
        <button class="text-button" id="auth-switch" type="button">${mode === "login" ? "Create an account" : "I already have an account"}</button>
        ${onboarding ? `<button class="text-button" id="auth-skip" type="button">Continue offline</button>` : ""}
        <span class="modal-hint">${isDesktopRuntime() ? "Secrets stay in the OS keychain" : "Browser preview"}</span>
        <button class="primary-button" type="submit">${mode === "login" ? "Sign in" : "Register"} ${icon("arrow-right")}</button>
      </div>
    </form>
  `;
  $("#close-auth").addEventListener("click", () => { gate.hidden = true; });
  $("#auth-switch").addEventListener("click", () => showAuthModal(mode === "login" ? "register" : "login", { onboarding }));
  $("#auth-skip")?.addEventListener("click", () => {
    localStorage.setItem("elyra-onboarding-skipped", "1");
    gate.hidden = true;
  });
  $("#auth-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const submit = form.querySelector("[type=submit]");
    const errorNode = form.querySelector("#auth-error");
    submit.disabled = true;
    errorNode.textContent = "";
    try {
      state.apiBase = await setApiBase(data.get("apiUrl"));
      await (mode === "login"
        ? loginAccount({ email: data.get("email"), password: data.get("password") })
        : registerAccount({ email: data.get("email"), password: data.get("password"), displayName: data.get("displayName") }));
      state.session = getSession();
      localStorage.removeItem("elyra-onboarding-skipped");
      gate.hidden = true;
      updateAccountUi();
      await refreshCloudMailStatus();
      renderSettings();
      await refreshNotifications().catch(() => {});
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
        <label>From<select name="accountId" id="compose-account"></select></label>
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
    renderComposeAccounts();
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
        await invoke("send_email", {
          to: data.get("to"),
          subject: data.get("subject"),
          body: data.get("body"),
          accountId: data.get("accountId") ? Number(data.get("accountId")) : null,
        });
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
  renderComposeAccounts();
  modal.hidden = false;
  form.elements.to.focus();
}

const ATTACHMENT_KIND_LABELS = {
  executable: "Executable",
  script: "Script",
  archive: "Archive",
  "disk-image": "Disk image",
  macro: "Macro document",
  document: "Document",
  image: "Image",
  other: "File",
};

const ATTACHMENT_RISK_LABELS = {
  safe: "Scanned",
  suspicious: "Suspicious",
  malicious: "Malicious",
  unknown: "Not scanned",
};

const ATTACHMENT_SOURCE_LABELS = {
  local: "Local scan",
  clamav: "ClamAV",
  virustotal: "VirusTotal",
};

function attachmentRisk(attachment) {
  const risk = String(attachment.risk || "").toLowerCase();
  return ["safe", "suspicious", "malicious"].includes(risk) ? risk : "unknown";
}

function attachmentCard(attachment) {
  const risk = attachmentRisk(attachment);
  const kind = ATTACHMENT_KIND_LABELS[attachment.kind] || ATTACHMENT_KIND_LABELS.other;
  const reasons = (attachment.reasons || []).map((reason) => `<li>${escapeHtml(reason)}</li>`).join("");
  const detected = attachment.detected_mime && attachment.detected_mime !== attachment.mime_type
    ? `${attachment.detected_mime} · declared ${attachment.mime_type}`
    : attachment.mime_type;
  const source = ATTACHMENT_SOURCE_LABELS[attachment.scan_source];
  return `
    <div class="attachment-card ${risk}" data-attachment-id="${attachment.id}">
      <span class="attachment-icon-wrap">${icon("paperclip", "attachment-icon")}</span>
      <div class="attachment-main">
        <div class="attachment-title">
          <b>${escapeHtml(attachment.name)}</b>
          <span class="attachment-risk ${risk}">${icon(risk === "safe" ? "shield" : "alert")} ${ATTACHMENT_RISK_LABELS[risk]}</span>
          <span class="attachment-kind">${escapeHtml(kind)}</span>
        </div>
        <small>${escapeHtml(detected)} · ${formatBytes(attachment.size_bytes)}${source ? ` · ${escapeHtml(source)}` : ""}</small>
        ${reasons ? `<ul class="attachment-reasons">${reasons}</ul>` : ""}
        ${attachment.content_available ? "" : `<small class="attachment-note">Content is not stored locally, so only metadata was checked.</small>`}
      </div>
      <div class="attachment-actions">
        <button type="button" data-scan-attachment title="Scan attachment">${icon("refresh")}</button>
        <button type="button" data-scan-virustotal title="Upload to VirusTotal with consent">${icon("shield")}</button>
        <button type="button" data-download-attachment title="Download attachment">${icon("download")}</button>
      </div>
    </div>
  `;
}

async function downloadAttachment(attachment) {
  const risk = attachmentRisk(attachment);
  if (risk === "malicious" && !window.confirm(`“${attachment.name}” is flagged as malicious. Download it anyway?`)) return;
  if (risk === "suspicious" && !window.confirm(`“${attachment.name}” looks suspicious. Download it anyway?`)) return;
  try {
    const content = await invoke("get_attachment_content", { id: attachment.id });
    const binary = atob(content.data_base64);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    const blob = new Blob([bytes], { type: content.mime_type || attachment.mime_type || "application/octet-stream" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = content.name || attachment.name;
    link.rel = "noopener";
    document.body.appendChild(link);
    link.click();
    link.remove();
    window.setTimeout(() => URL.revokeObjectURL(url), 2000);
    showToast("Downloaded. Elyra never opens attachments automatically.", "success");
  } catch (error) {
    showToast(error.message || "Could not download this attachment", "error");
  }
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
          <div class="viewer-labels">
            <span class="message-tag ${message.color}">${escapeHtml(message.tag)}</span>
            ${message.is_important ? `<span class="importance-chip">${icon("important")} Important</span>` : ""}
            ${priorityValue(message.priority) !== "normal" ? `<span class="priority-chip ${priorityValue(message.priority)}">${icon("priority")} ${priorityLabel(message.priority)} priority</span>` : ""}
            ${message.blocked_by ? `<span class="blocked-chip">${icon("spam")} ${escapeHtml(blockedRuleLabel(message.blocked_by))}</span>` : ""}
            <span class="secure-inline">${icon("lock")} local</span>
          </div>
          <h2>${escapeHtml(message.subject)}</h2>
          <p><b>${escapeHtml(message.sender)}</b> &lt;${escapeHtml(message.folder.toLowerCase() === "sent" ? detail.recipient : replyTo)}&gt;</p>
        </div>
        <span class="message-time">${escapeHtml(message.time)}</span>
      </div>
      <div class="viewer-actions">
        <button type="button" id="viewer-star" class="${message.is_starred ? "selected" : ""}">${icon("star")} ${message.is_starred ? "Starred" : "Star"}</button>
        <button type="button" id="viewer-important" class="${message.is_important ? "selected" : ""}">${icon("important")} ${message.is_important ? "Important" : "Mark important"}</button>
        ${isDraft ? `<button type="button" id="viewer-edit">${icon("edit")} Edit draft</button>` : `<button type="button" id="viewer-reply">${icon("reply")} Reply</button><button type="button" id="viewer-forward">${icon("forward")} Forward</button>`}
        <button type="button" id="viewer-unread">${icon("mail")} Mark unread</button>
      </div>
      <div class="viewer-priority">
        <label><span>Priority</span>
          <select id="viewer-priority-select">
            ${PRIORITY_ORDER.map((priority) => `<option value="${priority}" ${priorityValue(message.priority) === priority ? "selected" : ""}>${PRIORITY_LABELS[priority]}</option>`).join("")}
          </select>
        </label>
        <span class="priority-reason" id="viewer-priority-reason">${escapeHtml(prioritySourceLabel(message))}</span>
      </div>
      <div class="reputation-host" id="sender-reputation"></div>
      <div class="threat-host" id="message-shield"></div>
      <div class="viewer-body">${escapeHtml(detail.body_text).replace(/\n/g, "<br />")}</div>
      ${detail.attachments.length ? `<div class="attachment-list">${detail.attachments.map(attachmentCard).join("")}</div>` : ""}
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
    $("#viewer-important").addEventListener("click", async (event) => {
      const next = !message.is_important;
      await invoke("update_message", { id: message.id, isImportant: next }).catch(() => {});
      message.is_important = next;
      event.currentTarget.classList.toggle("selected", next);
      event.currentTarget.innerHTML = `${icon("important")} ${next ? "Important" : "Mark important"}`;
      const labels = viewer.querySelector(".viewer-labels");
      const tag = labels?.querySelector(".message-tag");
      if (labels && tag) {
        labels.querySelector(".importance-chip")?.remove();
        if (next) tag.insertAdjacentHTML("afterend", `<span class="importance-chip">${icon("important")} Important</span>`);
      }
      await refreshStats();
      await loadMailbox({ keepSelection: true });
    });
    $("#viewer-priority-select").addEventListener("change", async (event) => {
      const priority = priorityValue(event.target.value);
      await invoke("update_message", { id: message.id, priority }).catch(() => {});
      message.priority = priority;
      message.priority_source = "manual";
      message.priority_reason = "Set manually.";
      const reason = $("#viewer-priority-reason");
      if (reason) reason.textContent = "Set manually.";
      const labels = viewer.querySelector(".viewer-labels");
      const tag = labels?.querySelector(".message-tag");
      if (labels && tag) {
        labels.querySelector(".priority-chip")?.remove();
        if (priority !== "normal") tag.insertAdjacentHTML("afterend", `<span class="priority-chip ${priority}">${icon("priority")} ${priorityLabel(priority)} priority</span>`);
      }
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
    renderSenderReputation(viewer, message.id).catch(() => {});
    renderMessageShield(viewer, message.id).catch(() => {});
    $$("[data-scan-attachment]", viewer).forEach((button) => button.addEventListener("click", async () => {
      const attachmentId = Number(button.closest("[data-attachment-id]")?.dataset.attachmentId);
      button.disabled = true;
      try {
        await invoke("scan_attachment", { id: attachmentId });
        showToast("Attachment scanned", "success");
        await openMessage(message.id);
      } catch (error) {
        button.disabled = false;
        showToast(error.message || "Could not scan this attachment", "error");
      }
    }));
    $$("[data-scan-virustotal]", viewer).forEach((button) => button.addEventListener("click", async () => {
      const attachmentId = Number(button.closest("[data-attachment-id]")?.dataset.attachmentId);
      const attachment = detail.attachments.find((item) => Number(item.id) === attachmentId);
      if (!attachment) return;
      if (!window.confirm(`Upload “${attachment.name}” to VirusTotal?\n\nThis sends the file to a third party. Only continue if you trust the service and have permission to share this file.`)) return;
      button.disabled = true;
      try {
        await invoke("scan_attachment_external", { id: attachmentId, consent: true });
        showToast("VirusTotal scan finished", "success");
        await openMessage(message.id);
      } catch (error) {
        button.disabled = false;
        showToast(error.message || "Could not upload this attachment", "error");
      }
    }));
    $$("[data-download-attachment]", viewer).forEach((button) => button.addEventListener("click", () => {
      const attachmentId = Number(button.closest("[data-attachment-id]")?.dataset.attachmentId);
      const attachment = detail.attachments.find((item) => Number(item.id) === attachmentId);
      if (attachment) downloadAttachment(attachment);
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
    { label: "Show unread mail", hint: "Filter", icon: "mail", run: () => setQuickFilter("unread") },
    { label: "Show starred mail", hint: "Filter", icon: "star", run: () => setQuickFilter("starred") },
    { label: "Show important mail", hint: "Filter", icon: "important", run: () => setQuickFilter("important") },
    { label: "Show high-priority mail", hint: "Filter", icon: "priority", run: () => setQuickFilter("high") },
    { label: "Show urgent mail", hint: "Filter", icon: "alert", run: () => setQuickFilter("urgent") },
    { label: "Refresh priority suggestions", hint: "Local rules", icon: "refresh", run: refreshPrioritySuggestions },
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
    setQuickFilter(next);
  });

  $("#compose-button").addEventListener("click", () => openComposer());
  $("#refresh-button").addEventListener("click", (event) => refreshMailbox(event.currentTarget));
  $(".notification-button").addEventListener("click", openNotifications);
  $("#notification-button")?.addEventListener("click", openNotificationComposer);
  $("#connect-account")?.addEventListener("click", () => openAccountModal());
  $("#add-account")?.addEventListener("click", () => openAccountModal());
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
    if (event.target.closest("#save-api-url")) saveApiUrl();
    if (event.target.closest("#reset-demo")) resetDemo();
    if (event.target.closest("#export-mailbox")) exportMailbox();
    if (event.target.closest("#refresh-diagnostics")) renderDiagnostics();
    if (event.target.closest("#refresh-priority")) refreshPrioritySuggestions();
    if (event.target.closest("#add-sender-rule")) addSenderRule();
    if (event.target.closest("#apply-sender-rules")) applySenderRules();
    if (event.target.closest("#rescan-attachments")) rescanAttachments(event.target.closest("#rescan-attachments"));
    const deleteRule = event.target.closest("[data-delete-sender-rule]");
    if (deleteRule) deleteSenderRule(Number(deleteRule.dataset.deleteSenderRule));
    if (event.target.closest("#sign-out")) signOut();
    if (event.target.closest("#open-auth")) showAuthModal("login");
    if (event.target.closest("#add-mail-account")) openAccountModal();
    if (event.target.closest("#sync-all-accounts")) refreshMailbox($("#sync-all-accounts"));
  });

  document.addEventListener("click", (event) => {
    const action = event.target.closest("[data-account-action]");
    if (!action) return;
    const accountId = Number(action.dataset.accountId);
    if (action.dataset.accountAction === "edit") openAccountModal(accountId);
    if (action.dataset.accountAction === "sync") syncAccount(accountId, action);
    if (action.dataset.accountAction === "delete") {
      const account = state.accounts.find((item) => item.id === accountId);
      if (!account || !window.confirm(`Remove ${account.email} and its local messages from Elyra?`)) return;
      invoke("delete_email_account", { id: accountId })
        .then(async () => {
          await loadAccounts();
          await refreshStats();
          await loadMailbox();
          showToast("Account removed", "success");
        })
        .catch((error) => showToast(error.message || "Could not remove the account", "error"));
    }
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
  $("#blocked-sender-action")?.addEventListener("change", (event) => {
    setBlockedSenderAction(event.target.value);
  });
  ["#sender-rule-value", "#sender-rule-note"].forEach((selector) => {
    $(selector)?.addEventListener("keydown", (event) => {
      if (event.key !== "Enter") return;
      event.preventDefault();
      addSenderRule();
    });
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
      setQuickFilter("unread");
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
  const previousAuto = String(state.settings.auto_priority ?? "true");
  const previousDefault = state.settings.priority_default || "normal";
  const previousBlockedAction = state.settings.blocked_sender_action || "spam";
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
    auto_priority: $("#auto-priority").checked,
    priority_default: $("#priority-default").value,
    blocked_sender_action: $("#blocked-sender-action")?.value || "spam",
    attachment_scanning: $("#attachment-scanning").checked,
    attachment_clamav: $("#attachment-clamav").checked,
    clamav_path: $("#clamav-path").value.trim() || "clamscan",
    attachment_virustotal: $("#attachment-virustotal").checked,
    attachment_scan_max_mb: Math.min(200, Math.max(1, Number($("#attachment-scan-max").value) || 25)),
  };
  try {
    await invoke("set_local_settings", { values });
    state.settings = { ...state.settings, ...values };
    applyPreferences(state.settings);
    updateAccountUi();
    $("#save-settings").innerHTML = `${icon("check")} Saved locally`;
    window.setTimeout(() => { $("#save-settings").innerHTML = `${icon("check")} Save changes`; }, 1600);
    showToast("Settings saved", "success");
    if (String(values.auto_priority) !== previousAuto || values.priority_default !== previousDefault) {
      await refreshPrioritySuggestions({ quiet: true });
    }
    if (values.blocked_sender_action !== previousBlockedAction) {
      await applySenderRules({ quiet: true });
    }
  } catch (error) {
    showToast(error.message || "Could not save settings", "error");
  }
}

async function rescanAttachments(button) {
  if (button) button.disabled = true;
  try {
    const count = await invoke("scan_all_attachments");
    showToast(`${count} attachment${count === 1 ? "" : "s"} scanned`, "success");
    const viewer = $("#message-viewer");
    if (state.currentMessage && !viewer.hidden) {
      await openMessage(state.currentMessage.message.id);
    }
  } catch (error) {
    showToast(error.message || "Could not scan attachments", "error");
  } finally {
    if (button) button.disabled = false;
  }
}

async function refreshPrioritySuggestions({ quiet = false } = {}) {
  try {
    const count = await invoke("refresh_priority_suggestions");
    await refreshStats();
    await loadMailbox({ keepSelection: true });
    if (!quiet) {
      showToast(`Refreshed ${count} automatic priorit${count === 1 ? "y" : "ies"}`, "success");
    }
  } catch (error) {
    showToast(error.message || "Could not refresh priority suggestions", "error");
  }
}

async function saveApiUrl() {
  const input = $("#api-url-input");
  const button = $("#save-api-url");
  if (!input) return;
  try {
    state.apiBase = await setApiBase(input.value);
    input.value = state.apiBase;
    await refreshCloudMailStatus();
    renderSettings();
    if (button) {
      button.innerHTML = `${icon("check")} Server saved`;
      window.setTimeout(() => { button.innerHTML = `${icon("check")} Save server`; }, 1600);
    }
    showToast("Server URL saved", "success");
  } catch (error) {
    showToast(error.message || "Could not save the server URL", "error");
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
  state.apiBase = await getApiBase().catch(() => "");
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
  await loadAccounts();
  await refreshStats();
  await loadMailbox();
  await refreshCloudMailStatus();
  renderChanges();
  renderProfile();
  renderSettings();
  renderDiagnostics();
  if (!state.session && !localStorage.getItem("elyra-onboarding-skipped")) {
    showAuthModal("register", { onboarding: true });
  }
  refreshNotifications().catch(() => {});
  window.setInterval(() => refreshNotifications().catch(() => {}), 30000);
}

document.addEventListener("DOMContentLoaded", boot);
