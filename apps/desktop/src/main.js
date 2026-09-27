const messages = [
  { sender: "Maya Chen", initials: "MC", subject: "The quieter way to work", preview: "I’ve been thinking about the new rhythm we discussed...", time: "9:42 AM", tag: "Important", category: "important", color: "coral", unread: true, starred: true },
  { sender: "Elyra Security", initials: "ES", subject: "New sign-in protected", preview: "A new sign-in was protected with your device key.", time: "8:16 AM", tag: "Security", category: "security", color: "gold", unread: true, starred: false },
  { sender: "Linear", initials: "L", subject: "Your weekly product digest", preview: "Here’s what changed across your active projects this week.", time: "Yesterday", tag: "Updates", category: "updates", color: "teal", unread: true, starred: false },
  { sender: "Jon Bell", initials: "JB", subject: "Re: Q4 planning notes", preview: "The smaller scope feels right. I added two notes to the doc.", time: "Yesterday", tag: "Important", category: "important", color: "blue", unread: true, starred: false },
  { sender: "Noah Williams", initials: "NW", subject: "Coffee next week?", preview: "Tuesday or Thursday both work on my end.", time: "Sep 26", tag: "Personal", category: "inbox", color: "violet", unread: false, starred: true },
  { sender: "Daily Offers", initials: "DO", subject: "You have been selected", preview: "Claim your exclusive reward before midnight.", time: "Sep 25", tag: "Spam", category: "spam", color: "coral", unread: false, starred: false },
  { sender: "Maya Chen", initials: "MC", subject: "Old project notes", preview: "Moved to trash during inbox cleanup.", time: "Sep 18", tag: "Trash", category: "trash", color: "blue", unread: false, starred: false },
];

let currentFilter = "inbox";
let selectedMessage = null;

function renderMessages() {
  const query = document.querySelector("#search-input").value.toLowerCase();
  const list = document.querySelector("#message-list");
  const visible = messages.filter((message) => {
    const matchesFilter = currentFilter === "inbox" ? !["spam", "trash"].includes(message.category) : currentFilter === "starred" ? message.starred : message.category === currentFilter;
    const matchesSearch = `${message.sender} ${message.subject} ${message.preview}`.toLowerCase().includes(query);
    return matchesFilter && matchesSearch;
  });
  list.innerHTML = visible.length ? visible.map((message) => `<article class="message-row ${message.unread ? "unread" : ""}" data-message-id="${messages.indexOf(message)}"><input type="checkbox" aria-label="Select ${message.subject}" /><button class="star ${message.starred ? "selected" : ""}" aria-label="Star ${message.subject}" type="button">★</button><span class="sender-avatar ${message.color}">${message.initials}</span><div class="message-copy"><div><b>${message.sender}</b><span class="message-time">${message.time}</span></div><p><strong>${message.subject}</strong> <span>${message.preview}</span></p></div><span class="message-tag ${message.color}">${message.tag}</span></article>`).join("") : `<div class="empty-state"><span>⌕</span><h3>No messages found</h3><p>Try another search or mailbox.</p></div>`;
}

function showMessage(message) {
  selectedMessage = message;
  document.querySelector("#message-list").hidden = true;
  const viewer = document.querySelector("#message-viewer");
  viewer.hidden = false;
  viewer.innerHTML = `<div class="viewer-toolbar"><button class="back-button" id="close-viewer" type="button">← Back to ${currentFilter}</button><div><button class="viewer-icon" title="Archive" type="button">▣</button><button class="viewer-icon" title="Move to trash" id="viewer-trash" type="button">⌫</button><button class="viewer-icon" title="More actions" type="button">•••</button></div></div><div class="viewer-heading"><div class="sender-avatar ${message.color}">${message.initials}</div><div><h2>${message.subject}</h2><p><b>${message.sender}</b> &lt;${message.sender.toLowerCase().replace(" ", ".")}@example.com&gt;</p></div><span class="message-time">${message.time}</span></div><div class="viewer-actions"><button type="button" id="viewer-star">${message.starred ? "★ Starred" : "☆ Star"}</button><button type="button">↗ Reply</button><button type="button">↪ Forward</button></div><div class="viewer-body"><p>Hi Alex,</p><p>${message.preview} I wanted to share a little more context here so you can review it when you have a quiet moment.</p><div class="inline-image"><span>▧</span><b>Inline image preview</b><small>Rendered safely in local preview mode</small></div><p>Thanks,<br />${message.sender}</p></div><div class="attachment-card"><span>⌁</span><span><b>message-notes.pdf</b><small>PDF · 248 KB</small></span><button type="button">↓</button></div>`;
  document.querySelector("#close-viewer").addEventListener("click", () => { viewer.hidden = true; document.querySelector("#message-list").hidden = false; });
  document.querySelector("#viewer-star").addEventListener("click", (event) => { message.starred = !message.starred; event.currentTarget.textContent = message.starred ? "★ Starred" : "☆ Star"; renderMessages(); });
  document.querySelector("#viewer-trash").addEventListener("click", () => { message.category = "trash"; viewer.hidden = true; renderMessages(); document.querySelector("#message-list").hidden = false; });
}

window.addEventListener("DOMContentLoaded", () => {
  renderMessages();
  document.querySelector("#search-input").addEventListener("input", renderMessages);
  document.querySelector("#message-list").addEventListener("click", (event) => { const row = event.target.closest(".message-row"); if (row && !event.target.closest("button, input")) showMessage(messages[Number(row.dataset.messageId)]); });
  document.querySelectorAll("[data-filter]").forEach((item) => item.addEventListener("click", () => {
    document.querySelectorAll("[data-filter]").forEach((navItem) => navItem.classList.remove("active"));
    item.classList.add("active");
    currentFilter = item.dataset.filter;
    document.querySelector("#message-viewer").hidden = true;
    const isChanges = currentFilter === "changes";
    const isProfile = currentFilter === "profile";
    const isSettings = currentFilter === "settings";
    document.querySelector("#message-list").hidden = isChanges;
    document.querySelector("#changes-panel").hidden = !isChanges;
    document.querySelector("#profile-panel").hidden = !isProfile;
    document.querySelector("#settings-panel").hidden = !isSettings;
    document.querySelector("#view-title").textContent = isChanges ? "Changes" : isProfile ? "Profile" : isSettings ? "Settings" : item.textContent.trim().replace(/\d+$/, "");
    document.querySelector("#view-subtitle").textContent = isChanges ? "A record of what is ready and what comes next" : isProfile ? "Your identity and connected account overview" : isSettings ? "Control how Elyra works on this device" : "4 unread messages · Last synced just now";
    if (!isChanges && !isProfile && !isSettings) renderMessages();
  }));
  document.querySelector("#refresh-button").addEventListener("click", (event) => { event.currentTarget.classList.add("spin"); renderMessages(); setTimeout(() => event.currentTarget.classList.remove("spin"), 500); });
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
  document.querySelector("#save-settings").addEventListener("click", (event) => { event.currentTarget.textContent = "Saved locally"; setTimeout(() => { event.currentTarget.textContent = "Save changes"; }, 1400); });
  document.querySelector("#theme-select").addEventListener("change", (event) => { document.body.dataset.theme = event.target.value; });
});
