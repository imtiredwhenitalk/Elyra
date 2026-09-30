import { mountSecurityPage } from "./pages/SecurityPage.js";
import { invoke } from "./services/tauri.js";

let messages = [
  { sender: "Maya Chen", initials: "MC", subject: "The quieter way to work", preview: "I’ve been thinking about the new rhythm we discussed...", time: "9:42 AM", tag: "Important", category: "important", color: "coral", unread: true, starred: true },
  { sender: "Elyra Security", initials: "ES", subject: "New sign-in protected", preview: "A new sign-in was protected with your device key.", time: "8:16 AM", tag: "Security", category: "security", color: "gold", unread: true, starred: false },
  { sender: "Linear", initials: "L", subject: "Your weekly product digest", preview: "Here’s what changed across your active projects this week.", time: "Yesterday", tag: "Updates", category: "updates", color: "teal", unread: true, starred: false },
  { sender: "Jon Bell", initials: "JB", subject: "Re: Q4 planning notes", preview: "The smaller scope feels right. I added two notes to the doc.", time: "Yesterday", tag: "Important", category: "important", color: "blue", unread: true, starred: false },
  { sender: "Noah Williams", initials: "NW", subject: "Coffee next week?", preview: "Tuesday or Thursday both work on my end.", time: "Sep 26", tag: "Personal", category: "inbox", color: "violet", unread: false, starred: true },
  { sender: "Daily Offers", initials: "DO", subject: "You have been selected", preview: "Claim your exclusive reward before midnight.", time: "Sep 25", tag: "Spam", category: "spam", color: "coral", unread: false, starred: false },
  { sender: "Maya Chen", initials: "MC", subject: "Old project notes", preview: "Moved to trash during inbox cleanup.", time: "Sep 18", tag: "Trash", category: "trash", color: "blue", unread: false, starred: false },
  { sender: "Elyra Team", initials: "ET", subject: "Welcome to Elyra", preview: "Thanks for joining the Elyra community! We’re excited to have you.", time: "Sep 15", tag: "Welcome", category: "inbox", color: "gold", unread: false, starred: false },
  { sender: "Elyra Security", initials: "ES", subject: "Password changed successfully", preview: "Your password was changed successfully. If you did not make this change, please contact support immediately.", time: "Sep 12", tag: "Security", category: "security", color: "gold", unread: false, starred: false },
  { sender: "Linear", initials: "L", subject: "New project created", preview: "A new project has been created in your workspace.", time: "Sep 10", tag: "Updates", category: "updates", color: "teal", unread: false, starred: false },
  { sender: "Jon Bell", initials: "JB", subject: "Meeting rescheduled", preview: "The meeting has been rescheduled to next week.", time: "Sep 8", tag: "Important", category: "important", color: "blue", unread: false, starred: false }
];

let currentFilter = "inbox";
let quickFilter = "all";
let selectedMessage = null;
const settingsKey = "elyra-settings";
const draftsKey = "elyra-drafts";

function escapeHtml(value) {
  return String(value).replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "'": "&#39;",
    '"': "&quot;"
  }[character]));
}

function assetUrl(name) {
  return `/assets/${name}`;
}

function loadSettings() {
  try {
    return JSON.parse(localStorage.getItem(settingsKey) || "{}");
  } catch {
    return {};
  }
}

function saveSettings(settings) {
  localStorage.setItem(settingsKey, JSON.stringify(settings));
}

function loadDrafts() {
  try {
    return JSON.parse(localStorage.getItem(draftsKey) || "[]");
  } catch {
    return [];
  }
}

function saveDraft(draft) {
  const drafts = loadDrafts().filter((item) => item.id !== draft.id);
  localStorage.setItem(draftsKey, JSON.stringify([draft, ...drafts]));
}

function showToast(message, tone = "default") {
  const toast = document.createElement("div");
  toast.className = `toast ${tone}`;
  toast.textContent = message;
  document.body.append(toast);
  requestAnimationFrame(() => toast.classList.add("visible"));
  setTimeout(() => {
    toast.classList.remove("visible");
    setTimeout(() => toast.remove(), 220);
  }, 2600);
}

function openComposer(initial = {}) {
  let modal = document.querySelector("#compose-modal");
  if (!modal) {
    modal = document.createElement("div");
    modal.id = "compose-modal";
    modal.className = "modal-backdrop";
    modal.innerHTML = `<form class="compose-modal" id="compose-form"><div class="compose-modal-header"><div><span class="eyebrow">New message</span><h2>Write with intention</h2></div><button class="modal-close" id="close-compose" type="button" aria-label="Close">×</button></div><label>To<input name="to" type="email" autocomplete="email" required placeholder="name@example.com" /></label><label>Subject<input name="subject" required placeholder="A clear subject" /></label><label class="body-field">Message<textarea name="body" required rows="8" placeholder="Write your message..."></textarea></label><div class="compose-modal-footer"><button class="text-button" id="save-draft" type="button">Save draft</button><span class="compose-hint">Stored locally on this device</span><button class="send-button" type="submit">Send message <span>→</span></button></div></form>`;
    document.body.append(modal);
    modal.addEventListener("click", (event) => {
      if (event.target === modal || event.target.closest("#close-compose")) modal.remove();
    });
    modal.querySelector("#save-draft").addEventListener("click", () => {
      const form = modal.querySelector("#compose-form");
      const data = new FormData(form);
      saveDraft({ id: Date.now(), to: data.get("to"), subject: data.get("subject"), body: data.get("body") });
      modal.remove();
      showToast("Draft saved locally", "success");
    });
    modal.querySelector("#compose-form").addEventListener("submit", async (event) => {
      event.preventDefault();
      const form = event.currentTarget;
      const submit = form.querySelector("[type=submit]");
      const data = new FormData(form);
      submit.disabled = true;
      submit.textContent = "Sending...";
      try {
        await composeEmail(data.get("to"), data.get("subject"), data.get("body"));
        modal.remove();
        showToast("Message sent", "success");
      } catch (error) {
        submit.disabled = false;
        submit.innerHTML = 'Send message <span>→</span>';
        showToast("Could not send. Check SMTP settings.", "error");
        console.error("Failed to send email", error);
      }
    });
  }
  const form = modal.querySelector("#compose-form");
  form.elements.to.value = initial.to || "";
  form.elements.subject.value = initial.subject || "";
  form.elements.body.value = initial.body || "";
  modal.hidden = false;
  form.elements.to.focus();
}

function setPanelHidden(selector, hidden) {
  const panel = document.querySelector(selector);
  if (panel) panel.hidden = hidden;
}

function normalizeAssetUrls() {
  document.querySelectorAll('img[src^="/src/assets/"]').forEach((image) => {
    image.src = image.getAttribute("src").replace("/src/assets/", "/assets/");
  });
}

async function sendEmail(to, subject, body) {
  await invoke("send_email", { to, subject, body });
  alert("Email sent");
}

async function listSentEmails() {
  const sentEmails = await invoke("list_sent_emails");
  console.log("Sent emails:", sentEmails);
  return sentEmails;
}

async function composeEmail(to, subject, body) {
  await sendEmail(to, subject, body);
  const sentEmails = await listSentEmails();
  messages = [...messages.filter((message) => message.category !== "sent"), ...sentEmails.map(mapSentEmail)];
  renderMessages();
  console.log("Updated sent emails:", sentEmails);
}

function renderMessages() {
  const query = document.querySelector("#search-input").value.toLowerCase();
  const list = document.querySelector("#message-list");
  const visible = messages.filter((message) => {
    const matchesFilter = currentFilter === "inbox" ? !["spam", "trash"].includes(message.category) : currentFilter === "starred" ? message.starred : message.category === currentFilter;
    const matchesQuickFilter = quickFilter === "all" || quickFilter === "unread" && message.unread || quickFilter === "starred" && message.starred;
    const matchesSearch = `${message.sender} ${message.subject} ${message.preview}`.toLowerCase().includes(query);
    return matchesFilter && matchesQuickFilter && matchesSearch;
  });
  list.innerHTML = visible.length ? visible.map((message) => `<article class="message-row ${message.unread ? "unread" : ""}" data-message-id="${messages.indexOf(message)}"><input type="checkbox" ${message.unread ? "" : "checked"} aria-label="Mark ${escapeHtml(message.subject)} as read" /><button class="star ${message.starred ? "selected" : ""}" aria-label="Star ${escapeHtml(message.subject)}" type="button">★</button><span class="sender-avatar ${message.color}">${escapeHtml(message.initials)}</span><div class="message-copy"><div><b>${escapeHtml(message.sender)}</b><span class="message-time">${escapeHtml(message.time)}</span></div><p><strong>${escapeHtml(message.subject)}</strong> <span>${escapeHtml(message.preview)}</span></p></div><span class="message-tag ${message.color}">${escapeHtml(message.tag)}</span></article>`).join("") : `<div class="empty-state"><span>⌕</span><h3>No messages found</h3><p>Try another search or mailbox.</p></div>`;
}

function mapCachedEmail(email) {
  const sender = email.sender || "Unknown sender";
  const initials = sender.split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase();
  const colors = ["coral", "gold", "teal", "blue", "violet"];
  const color = colors[Math.abs(sender.length + email.id) % colors.length];
  return {
    sender,
    initials,
    subject: email.subject,
    preview: email.body_text,
    time: "Just now",
    tag: email.folder,
    category: email.folder.toLowerCase(),
    color,
    unread: !email.is_read,
    starred: email.is_starred
  };
}

function mapSentEmail(email) {
  return {
    sender: "Alex Smith",
    initials: "AS",
    subject: email.subject,
    preview: email.body,
    time: "Just now",
    tag: "Sent",
    category: "sent",
    color: "blue",
    unread: false,
    starred: false
  };
}

async function refreshMessages(button) {
  button.disabled = true;
  button.classList.add("spin");
  try {
    const cachedEmails = await invoke("list_cached_emails");
    if (cachedEmails.length) messages = cachedEmails.map(mapCachedEmail);
    renderMessages();
    document.querySelector("#view-subtitle").textContent = `${messages.filter((message) => message.unread).length} unread messages · Last synced just now`;
  } catch (error) {
    console.error("Failed to refresh messages", error);
  } finally {
    button.disabled = false;
    button.classList.remove("spin");
  }
}

function showMessage(message) {
  selectedMessage = message;
  document.querySelector("#message-list").hidden = true;
  const viewer = document.querySelector("#message-viewer");
  viewer.hidden = false;
  const senderEmail = `${message.sender.toLowerCase().replace(" ", ".")}@example.com`;
  viewer.innerHTML = `<div class="viewer-toolbar"><button class="back-button" id="close-viewer" type="button">← Back to ${escapeHtml(currentFilter)}</button><div><button class="viewer-icon" title="Archive" type="button"><img class="action-icon" src="${assetUrl("archive.svg")}" alt="" /></button><button class="viewer-icon" title="Move to trash" id="viewer-trash" type="button"><img class="action-icon" src="${assetUrl("trash.svg")}" alt="" /></button><button class="viewer-icon" title="More actions" type="button">•••</button></div></div><div class="viewer-heading"><div class="sender-avatar ${message.color}">${escapeHtml(message.initials)}</div><div><h2>${escapeHtml(message.subject)}</h2><p><b>${escapeHtml(message.sender)}</b> &lt;${escapeHtml(senderEmail)}&gt;</p></div><span class="message-time">${escapeHtml(message.time)}</span></div><div class="viewer-actions"><button type="button" id="viewer-star">${message.starred ? "★ Starred" : "☆ Star"}</button><button type="button" id="viewer-reply">↗ Reply</button><button type="button">↪ Forward</button></div><div class="viewer-body"><p>Hi Alex,</p><p>${escapeHtml(message.preview)} I wanted to share a little more context here so you can review it when you have a quiet moment.</p><div class="inline-image"><span>▧</span><b>Inline image preview</b><small>Rendered safely in local preview mode</small></div><p>Thanks,<br />${escapeHtml(message.sender)}</p></div><div class="attachment-card"><img class="attachment-icon" src="${assetUrl("clip.svg")}" alt="" /><span><b>message-notes.pdf</b><small>PDF · 248 KB</small></span><button type="button" title="Download attachment">↓</button></div>`;
  document.querySelector("#close-viewer").addEventListener("click", () => { viewer.hidden = true; document.querySelector("#message-list").hidden = false; });
  document.querySelector("#viewer-star").addEventListener("click", (event) => { message.starred = !message.starred; event.currentTarget.textContent = message.starred ? "★ Starred" : "☆ Star"; renderMessages(); });
  document.querySelector("#viewer-reply").addEventListener("click", async () => {
    const body = window.prompt("Write your reply", "");
    if (body) {
      try {
        await composeEmail(senderEmail, `Re: ${message.subject}`, body);
      } catch (error) {
        console.error("Failed to send reply", error);
        alert("Unable to send the reply. Check your SMTP settings.");
      }
    }
  });
  document.querySelector("#viewer-trash").addEventListener("click", () => { message.category = "trash"; viewer.hidden = true; renderMessages(); document.querySelector("#message-list").hidden = false; });
}

window.addEventListener("DOMContentLoaded", () => {
  const settings = loadSettings();
  mountSecurityPage(document.querySelector("#settings-panel"));
  const theme = settings.theme || "light";
  document.body.dataset.theme = theme;
  document.querySelector("#theme-select").value = theme;
  const displayNameInput = document.querySelector('.settings-form input:not([type="checkbox"])');
  if (settings.displayName && displayNameInput) displayNameInput.value = settings.displayName;
  renderMessages();
  normalizeAssetUrls();
  document.addEventListener("click", normalizeAssetUrls);
  document.querySelector("#search-input").addEventListener("input", renderMessages);
  document.querySelector("#filter-button").addEventListener("click", (event) => {
    const filters = ["all", "unread", "starred"];
    quickFilter = filters[(filters.indexOf(quickFilter) + 1) % filters.length];
    event.currentTarget.firstChild.textContent = `${quickFilter[0].toUpperCase()}${quickFilter.slice(1)} `;
    renderMessages();
    showToast(`Showing ${quickFilter} messages`);
  });
  document.querySelector("#compose-button").addEventListener("click", () => openComposer());
  document.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      document.querySelector("#search-input").focus();
    }
    if (event.key === "Escape") document.querySelector("#compose-modal")?.remove();
  });
  document.querySelector("#message-list").addEventListener("change", (event) => {
    const row = event.target.closest(".message-row");
    if (row) {
      const message = messages[Number(row.dataset.messageId)];
      if (event.target.type === "checkbox") {
        message.unread = !event.target.checked;
        renderMessages();
      }
    }
  });
  document.querySelector("#message-list").addEventListener("click", (event) => { const row = event.target.closest(".message-row"); if (row && !event.target.closest("button, input")) showMessage(messages[Number(row.dataset.messageId)]); });
  document.querySelector("#message-list").addEventListener("click", (event) => {
    const star = event.target.closest(".star");
    if (!star) return;
    const row = star.closest(".message-row");
    const message = row && messages[Number(row.dataset.messageId)];
    if (!message) return;
    message.starred = !message.starred;
    renderMessages();
  });
  document.querySelectorAll("[data-filter]").forEach((item) => item.addEventListener("click", () => {
    document.querySelectorAll("[data-filter]").forEach((navItem) => navItem.classList.remove("active"));
    item.classList.add("active");
    currentFilter = item.dataset.filter;
    document.querySelector("#message-viewer").hidden = true;
    const isChanges = currentFilter === "changes";
    const isProfile = currentFilter === "profile";
    const isSettings = currentFilter === "settings";
    const isInbox = currentFilter === "inbox";
    const isStarred = currentFilter === "starred";
    const isSpam = currentFilter === "spam";
    const isTrash = currentFilter === "trash";
    const isImportant = currentFilter === "important";
    const isUpdates = currentFilter === "updates";
    const isPersonal = currentFilter === "personal";
    const compose = currentFilter === "compose";
      const isDedicatedPanel = isChanges || isProfile || isSettings || compose;
      document.querySelector("#message-list").hidden = isDedicatedPanel;
      setPanelHidden("#changes-panel", !isChanges);
      setPanelHidden("#profile-panel", !isProfile);
      setPanelHidden("#settings-panel", !isSettings);
      setPanelHidden("#compose-panel", !compose);
      setPanelHidden("#inbox-panel", !isInbox);
      setPanelHidden("#starred-panel", !isStarred);
      setPanelHidden("#spam-panel", !isSpam);
      setPanelHidden("#trash-panel", !isTrash);
      setPanelHidden("#important-panel", !isImportant);
      setPanelHidden("#updates-panel", !isUpdates);
      setPanelHidden("#personal-panel", !isPersonal);
    document.querySelector("#view-subtitle").hidden = isChanges || isProfile || isSettings;
    document.querySelector("#view-title").textContent = isChanges ? "Changes" : isProfile ? "Profile" : isSettings ? "Settings" : item.textContent.trim().replace(/\d+$/, "");
    document.querySelector("#view-subtitle").textContent = isChanges ? "A record of what is ready and what comes next" : isProfile ? "Your identity and connected account overview" : isSettings ? "Control how Elyra works on this device" : "4 unread messages · Last synced just now";
    if (!isDedicatedPanel) renderMessages();
  }));
  document.querySelector("#refresh-button").addEventListener("click", (event) => refreshMessages(event.currentTarget));
  document.querySelector(".top-actions .icon-button").addEventListener("click", () => showToast("You are all caught up", "success"));
  document.querySelector(".top-actions .avatar").addEventListener("click", () => openSettings());
  document.querySelectorAll("#connect-account, #add-account").forEach((button) => button.addEventListener("click", () => alert("OAuth connection flow will be added in the next integration step.")));
  const openSettings = () => {
    document.querySelectorAll("[data-filter]").forEach((navItem) => navItem.classList.remove("active"));
    currentFilter = "settings";
    document.querySelector("#message-list").hidden = true;
    document.querySelector("#changes-panel").hidden = true;
    document.querySelector("#profile-panel").hidden = true;
    document.querySelector("#settings-panel").hidden = false;
    document.querySelector("#view-title").textContent = "Settings";
    document.querySelector("#view-subtitle").textContent = "Control how Elyra works on this device";
  };
  document.querySelector("#settings-button").addEventListener("click", openSettings);
  document.querySelector("#profile-settings").addEventListener("click", openSettings);
  document.querySelector("#save-settings").addEventListener("click", (event) => {
    const nextSettings = { ...loadSettings(), theme: document.querySelector("#theme-select").value, displayName: displayNameInput ? displayNameInput.value : "" };
    saveSettings(nextSettings);
    event.currentTarget.textContent = "Saved locally";
    setTimeout(() => { event.currentTarget.textContent = "Save changes"; }, 1400);
  });
  document.querySelector("#theme-select").addEventListener("change", (event) => { document.body.dataset.theme = event.target.value; });
});
