const FALLBACK_STORAGE_PREFIX = "elyra-fallback";
const MAILBOX_STORE = "mailbox-v2";
const SETTINGS_STORE = "settings-v1";

function readStore(key, fallback) {
  try {
    const raw = window.localStorage.getItem(`${FALLBACK_STORAGE_PREFIX}:${key}`);
    return raw ? JSON.parse(raw) : fallback;
  } catch {
    return fallback;
  }
}

function writeStore(key, value) {
  try {
    window.localStorage.setItem(`${FALLBACK_STORAGE_PREFIX}:${key}`, JSON.stringify(value));
  } catch {
    // Browser preview can run with storage disabled; the session stays in memory.
  }
}

function toBase64(bytes) {
  let binary = "";
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return window.btoa(binary);
}

function fromBase64(value) {
  const binary = window.atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

const COLORS = ["coral", "gold", "teal", "blue", "violet", "moss"];

function initialsFor(sender) {
  const initials = String(sender)
    .split(/[^a-zA-Z0-9]+/)
    .filter(Boolean)
    .map((part) => part[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
  return initials || "?";
}

function colorFor(sender) {
  const seed = [...String(sender)].reduce((sum, character) => sum + character.charCodeAt(0), 0);
  return COLORS[seed % COLORS.length];
}

function sqlTime(offsetMinutes = 0) {
  return new Date(Date.now() - offsetMinutes * 60_000).toISOString().slice(0, 19).replace("T", " ");
}

function displayTime(receivedAt) {
  const parsed = new Date(`${String(receivedAt).replace(" ", "T")}Z`);
  if (Number.isNaN(parsed.getTime())) return receivedAt;
  const delta = Date.now() - parsed.getTime();
  if (delta < 60_000) return "Just now";
  if (delta < 3_600_000) return `${Math.max(1, Math.floor(delta / 60_000))}m ago`;
  const now = new Date();
  if (parsed.toDateString() === now.toDateString()) {
    return parsed.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  if (new Date(now.getTime() - 86_400_000).toDateString() === parsed.toDateString()) return "Yesterday";
  return parsed.toLocaleDateString([], { month: "short", day: "numeric" });
}

function previewOf(body) {
  const compact = String(body).split(/\s+/).join(" ").trim();
  return compact.length > 150 ? `${compact.slice(0, 147)}...` : compact;
}

function demoMessages() {
  const rows = [
    ["Maya Chen", "alex@elyra.com", "The quieter way to work", 12, "Important", "Important", false, true, true, "Hi Alex,\n\nI've been thinking about the new rhythm we discussed. Fewer meetings, longer focus blocks, and a calmer inbox would give the team room to do their best work.\n\nI put together a short proposal with three experiments we could run next month.\n\nMaya"],
    ["Elyra Security", "alex@elyra.com", "New sign-in protected", 48, "Security", "Security", false, false, false, "A new sign-in was protected with your device key.\n\nDevice: Windows 11 desktop\nLocation: this device\n\nIf this was you, no action is needed."],
    ["Linear", "alex@elyra.com", "Your weekly product digest", 220, "Updates", "Updates", false, false, false, "Here is what changed across your active projects this week.\n\n- Inbox workspace reached the review milestone\n- 14 issues were completed\n- The privacy settings pass is ready for testing"],
    ["Jon Bell", "alex@elyra.com", "Re: Q4 planning notes", 330, "Important", "Important", false, true, true, "The smaller scope feels right. I added two notes to the doc: keep the first release focused on the desktop inbox, and move provider sync behind a feature flag."],
    ["Noah Williams", "alex@elyra.com", "Coffee next week?", 1_440, "Inbox", "Personal", true, true, false, "Tuesday or Thursday both work on my end. There is a new place near the studio with a quiet back room and very good cardamom buns."],
    ["Daily Offers", "alex@elyra.com", "You have been selected", 1_600, "Spam", "Spam", true, false, false, "Claim your exclusive reward before midnight. This message was automatically filtered into Spam."],
    ["Maya Chen", "alex@elyra.com", "Old project notes", 8_300, "Trash", "Trash", true, false, false, "Moved to trash during inbox cleanup."],
    ["Elyra Team", "alex@elyra.com", "Welcome to Elyra", 9_100, "Inbox", "Welcome", true, false, false, "Thanks for joining the Elyra community. Your inbox is now a local-first workspace."],
    ["Elyra Security", "alex@elyra.com", "Password changed successfully", 12_200, "Security", "Security", true, false, false, "Your password was changed successfully. If you did not make this change, contact support immediately."],
    ["Linear", "alex@elyra.com", "New project created", 14_000, "Updates", "Updates", true, false, false, "A new project has been created in your workspace."],
    ["Jon Bell", "alex@elyra.com", "Meeting rescheduled", 16_000, "Important", "Important", true, false, false, "The meeting has been moved to next week."],
    ["Proton", "alex@elyra.com", "Your weekly privacy report", 95, "Security", "Security", false, false, false, "This week Elyra blocked 18 trackers, kept 42 messages local, and verified every encryption round-trip."],
    ["GitHub", "alex@elyra.com", "Release 0.2.0 is ready", 720, "Updates", "Updates", true, false, false, "The new desktop build includes the Rust mailbox engine, a refreshed interface, a proper Windows installer and an improved website."],
    ["Sofia Marin", "alex@elyra.com", "Design review notes", 180, "Inbox", "Personal", false, false, false, "The hierarchy is much calmer now. My only ask is to keep the reading pane generous on wide screens and collapse it gracefully on laptops."],
    ["Elyra Team", "alex@elyra.com", "Snoozed until tomorrow", 2_000, "Snoozed", "Snoozed", true, false, false, "This message was snoozed for one day so the inbox stays focused."],
    ["Alex Smith", "maya@example.com", "Re: The quieter way to work", 25, "Sent", "Sent", true, false, false, "Maya,\n\nI love the direction. Let's run the focus-block experiment first and keep the rest lightweight.\n\nAlex"],
    ["Alex Smith", "team@elyra.com", "Launch notes", 40, "Drafts", "Draft", true, false, false, "Draft outline:\n\n1. Rust mailbox core\n2. New icon system\n3. MSI release pipeline\n4. Website refresh"],
    ["Tomas Adler", "alex@elyra.com", "Archived: office move checklist", 20_000, "Archive", "Archive", true, false, false, "Everything from the old office is packed. The archive copy is here for reference."],
  ];
  return rows.map((row, index) => ({
    id: index + 1,
    sender: row[0],
    recipient: row[1],
    subject: row[2],
    received_at: sqlTime(row[3]),
    folder: row[4],
    tag: row[5],
    is_read: row[6],
    is_starred: row[7],
    has_attachments: row[8],
    body_text: row[9],
    preview: previewOf(row[9]),
    attachments: row[8] ? [{ id: index + 1, name: "message-notes.pdf", mime_type: "application/pdf", size_bytes: 248_320 }] : [],
  }));
}

function loadMailbox() {
  let messages = readStore(MAILBOX_STORE, null);
  if (!Array.isArray(messages) || !messages.length) {
    messages = demoMessages();
    writeStore(MAILBOX_STORE, messages);
  }
  return messages;
}

function saveMailbox(messages) {
  writeStore(MAILBOX_STORE, messages);
}

function summaryOf(message) {
  return {
    id: message.id,
    sender: message.sender,
    initials: initialsFor(message.sender),
    subject: message.subject,
    preview: message.preview || previewOf(message.body_text),
    time: displayTime(message.received_at),
    received_at: message.received_at,
    folder: message.folder,
    tag: message.tag,
    color: colorFor(message.sender),
    is_read: Boolean(message.is_read),
    is_starred: Boolean(message.is_starred),
    has_attachments: Boolean(message.has_attachments),
    snoozed_until: message.snoozed_until || null,
  };
}

function statsOf(messages) {
  const folderCounts = {};
  const folderUnread = {};
  for (const message of messages) {
    const folder = String(message.folder || "Inbox").toLowerCase();
    folderCounts[folder] = (folderCounts[folder] || 0) + 1;
    if (!message.is_read) folderUnread[folder] = (folderUnread[folder] || 0) + 1;
  }
  return {
    total: messages.length,
    unread: messages.filter((message) => !message.is_read).length,
    starred: messages.filter((message) => message.is_starred).length,
    attachments: messages.reduce((sum, message) => sum + (message.attachments?.length || 0), 0),
    trash: folderCounts.trash || 0,
    spam: folderCounts.spam || 0,
    drafts: folderCounts.drafts || 0,
    snoozed: folderCounts.snoozed || 0,
    accounts: 3,
    storage_bytes: 2_400_000 + messages.length * 4_096,
    last_sync_at: readStore(SETTINGS_STORE, {}).last_sync_at || sqlTime(),
    folder_counts: folderCounts,
    folder_unread: folderUnread,
  };
}

function filterMessages(messages, filter, quickFilter, query) {
  const normalizedFilter = String(filter || "inbox").toLowerCase();
  const normalizedQuick = String(quickFilter || "all").toLowerCase();
  const needle = String(query || "").trim().toLowerCase();
  return messages.filter((message) => {
    const folder = String(message.folder).toLowerCase();
    if (normalizedFilter === "inbox") {
      if (["spam", "trash", "sent", "drafts", "snoozed", "archive"].includes(folder)) return false;
      if (message.snoozed_until && message.snoozed_until > sqlTime()) return false;
    } else if (normalizedFilter === "starred") {
      if (!message.is_starred || ["trash", "spam"].includes(folder)) return false;
    } else if (normalizedFilter && normalizedFilter !== "all" && folder !== normalizedFilter) {
      return false;
    }
    if (normalizedQuick === "unread" && message.is_read) return false;
    if (normalizedQuick === "starred" && !message.is_starred) return false;
    if (normalizedQuick === "attachments" && !message.has_attachments) return false;
    if (needle) {
      const haystack = `${message.sender} ${message.subject} ${message.preview} ${message.body_text}`.toLowerCase();
      if (!haystack.includes(needle)) return false;
    }
    return true;
  }).sort((a, b) => String(b.received_at).localeCompare(String(a.received_at)));
}

function pageOf(messages, filter, quickFilter, limit = 100, offset = 0) {
  const visible = filterMessages(messages, filter, quickFilter, "");
  const page = visible.slice(offset, offset + limit).map(summaryOf);
  const stats = statsOf(messages);
  return {
    messages: page,
    total: visible.length,
    unread: stats.unread,
    starred: stats.starred,
    has_more: offset + page.length < visible.length,
    filter,
    quick_filter: quickFilter,
  };
}

function nextId(messages) {
  return messages.reduce((max, message) => Math.max(max, Number(message.id) || 0), 0) + 1;
}

function loadSettings() {
  return readStore(SETTINGS_STORE, {});
}

function saveSettings(values) {
  writeStore(SETTINGS_STORE, { ...loadSettings(), ...values });
}

async function generateFallbackKey() {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return toBase64([...bytes]);
}

async function fallbackEncrypt(key, plaintext) {
  const keyBytes = fromBase64(key);
  const cryptoKey = await crypto.subtle.importKey("raw", keyBytes, "AES-GCM", false, ["encrypt"]);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const ciphertext = new Uint8Array(
    await crypto.subtle.encrypt({ name: "AES-GCM", iv }, cryptoKey, new TextEncoder().encode(plaintext)),
  );
  const payload = new Uint8Array(iv.length + ciphertext.length);
  payload.set(iv, 0);
  payload.set(ciphertext, iv.length);
  return toBase64([...payload]);
}

async function fallbackDecrypt(key, payload) {
  const keyBytes = fromBase64(key);
  const decoded = fromBase64(payload);
  if (decoded.length <= 12) throw new Error("Encrypted payload is incomplete");
  const iv = decoded.slice(0, 12);
  const ciphertext = decoded.slice(12);
  const cryptoKey = await crypto.subtle.importKey("raw", keyBytes, "AES-GCM", false, ["decrypt"]);
  const plainTex = await crypto.subtle.decrypt({ name: "AES-GCM", iv }, cryptoKey, ciphertext);
  return new TextDecoder().decode(plainTex);
}

async function fallbackFingerprint(value) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function changelog() {
  return [
    { version: "0.2.0", date: "2026-11-14", title: "Rust mailbox core", detail: "Filtering, search, folders, drafts, bulk actions, export and mailbox statistics now run in the Rust core.", status: "ready" },
    { version: "0.2.0", date: "2026-11-14", title: "Presentable interface refresh", detail: "Consistent SVG icons, richer themes, accent colours, density controls, command palette and a calmer reading pane.", status: "ready" },
    { version: "0.2.0", date: "2026-11-14", title: "Installer and release pipeline", detail: "MSI release workflow, stable download names, a Rust installer launcher and a refreshed website.", status: "ready" },
    { version: "next", date: "planned", title: "Provider sync", detail: "Gmail and Outlook OAuth with PKCE, IMAP fetch and OS keychain token storage are next.", status: "next" },
  ];
}

function fallbackInvoke(command, args = {}) {
  switch (command) {
    case "initialize_local_storage":
    case "set_local_setting":
    case "delete_local_setting":
      if (command === "set_local_setting") {
        saveSettings({ [args.key]: args.value });
      }
      return Promise.resolve();
    case "get_local_settings":
      return Promise.resolve(loadSettings());
    case "set_local_settings":
      saveSettings(args.values || {});
      return Promise.resolve();
    case "mailbox_stats":
      return Promise.resolve(statsOf(loadMailbox()));
    case "seed_demo_mailbox":
      if (args.force) {
        const messages = demoMessages();
        saveMailbox(messages);
        return Promise.resolve(statsOf(messages));
      }
      return Promise.resolve(statsOf(loadMailbox()));
    case "reset_demo_mailbox": {
      const messages = demoMessages();
      saveMailbox(messages);
      return Promise.resolve(statsOf(messages));
    }
    case "refresh_mailbox": {
      saveSettings({ last_sync_at: sqlTime() });
      return Promise.resolve(statsOf(loadMailbox()));
    }
    case "list_mailbox": {
      const messages = loadMailbox();
      const limit = args.limit || 100;
      const offset = args.offset || 0;
      const filtered = filterMessages(messages, args.filter, args.quickFilter, args.query);
      const page = filtered.slice(offset, offset + limit).map(summaryOf);
      const stats = statsOf(messages);
      return Promise.resolve({
        messages: page,
        total: filtered.length,
        unread: stats.unread,
        starred: stats.starred,
        has_more: offset + page.length < filtered.length,
        filter: args.filter || "inbox",
        quick_filter: args.quickFilter || "all",
      });
    }
    case "get_message": {
      const messages = loadMailbox();
      const message = messages.find((item) => Number(item.id) === Number(args.id));
      if (!message) return Promise.reject(new Error("Message not found"));
      message.is_read = true;
      saveMailbox(messages);
      return Promise.resolve({
        message: summaryOf(message),
        recipient: message.recipient || "",
        body_text: message.body_text || "",
        body_html: message.body_html || null,
        attachments: message.attachments || [],
      });
    }
    case "update_message": {
      const messages = loadMailbox();
      const message = messages.find((item) => Number(item.id) === Number(args.id));
      if (!message) return Promise.reject(new Error("Message not found"));
      if (args.isRead !== undefined) message.is_read = Boolean(args.isRead);
      if (args.isStarred !== undefined) message.is_starred = Boolean(args.isStarred);
      if (args.folder) message.folder = args.folder;
      saveMailbox(messages);
      return Promise.resolve();
    }
    case "delete_message": {
      let messages = loadMailbox();
      if (args.permanent) {
        messages = messages.filter((item) => Number(item.id) !== Number(args.id));
      } else {
        const message = messages.find((item) => Number(item.id) === Number(args.id));
        if (message) message.folder = "Trash";
      }
      saveMailbox(messages);
      return Promise.resolve();
    }
    case "bulk_update_messages": {
      let messages = loadMailbox();
      const ids = new Set((args.ids || []).map(Number));
      const action = String(args.action || "").toLowerCase();
      if (action === "delete") {
        messages = messages.filter((item) => !ids.has(Number(item.id)));
      } else {
        for (const message of messages) {
          if (!ids.has(Number(message.id))) continue;
          if (action === "read") message.is_read = true;
          if (action === "unread") message.is_read = false;
          if (action === "star") message.is_starred = true;
          if (action === "unstar") message.is_starred = false;
          if (action === "trash") message.folder = "Trash";
          if (action === "spam") message.folder = "Spam";
          if (action === "archive") message.folder = "Archive";
          if (action === "inbox" || action === "restore") message.folder = "Inbox";
          if (action === "snooze") {
            message.folder = "Snoozed";
            message.snoozed_until = args.until || sqlTime(-1_440);
          }
        }
      }
      saveMailbox(messages);
      return Promise.resolve(ids.size);
    }
    case "mark_all_read": {
      const messages = loadMailbox();
      const filtered = filterMessages(messages, args.filter || "inbox", "all", "");
      const visibleIds = new Set(filtered.map((message) => Number(message.id)));
      for (const message of messages) {
        if (visibleIds.has(Number(message.id))) message.is_read = true;
      }
      saveMailbox(messages);
      return Promise.resolve(visibleIds.size);
    }
    case "empty_trash": {
      const messages = loadMailbox().filter((message) => String(message.folder).toLowerCase() !== "trash");
      saveMailbox(messages);
      return Promise.resolve(1);
    }
    case "save_draft": {
      const messages = loadMailbox();
      let message = messages.find((item) => Number(item.id) === Number(args.id));
      if (!message) {
        message = {
          id: nextId(messages),
          sender: "Alex Smith",
          received_at: sqlTime(),
          folder: "Drafts",
          tag: "Draft",
          is_read: true,
          is_starred: false,
          has_attachments: false,
          attachments: [],
        };
        messages.unshift(message);
      }
      message.recipient = args.recipient || "";
      message.subject = args.subject || "(no subject)";
      message.body_text = args.body || "";
      message.preview = previewOf(message.body_text);
      saveMailbox(messages);
      return Promise.resolve(Number(message.id));
    }
    case "delete_draft": {
      const messages = loadMailbox().filter((message) => Number(message.id) !== Number(args.id));
      saveMailbox(messages);
      return Promise.resolve(true);
    }
    case "search_suggestions": {
      const needle = String(args.query || "").toLowerCase();
      if (!needle) return Promise.resolve([]);
      const messages = loadMailbox();
      const items = new Set();
      for (const message of messages) {
        if (message.sender.toLowerCase().includes(needle)) items.add(message.sender);
        if (message.subject.toLowerCase().includes(needle)) items.add(message.subject);
      }
      return Promise.resolve([...items].slice(0, args.limit || 6));
    }
    case "export_mailbox":
      return Promise.resolve({
        format: "elyra-mailbox",
        version: "0.2.0",
        exported_at: sqlTime(),
        messages: loadMailbox(),
        settings: loadSettings(),
      });
    case "send_email": {
      const messages = loadMailbox();
      messages.unshift({
        id: nextId(messages),
        sender: "Alex Smith",
        recipient: args.to,
        subject: args.subject,
        body_text: args.body,
        preview: previewOf(args.body),
        folder: "Sent",
        tag: "Sent",
        is_read: true,
        is_starred: false,
        has_attachments: false,
        attachments: [],
        received_at: sqlTime(),
      });
      saveMailbox(messages);
      return Promise.resolve();
    }
    case "list_sent_emails":
      return Promise.resolve(
        loadMailbox()
          .filter((message) => String(message.folder).toLowerCase() === "sent")
          .map((message) => ({ to: message.recipient, subject: message.subject, body: message.body_text })),
      );
    case "get_changelog":
      return Promise.resolve(changelog());
    case "get_app_diagnostics": {
      const stats = statsOf(loadMailbox());
      return Promise.resolve({
        app_version: "0.2.0",
        engine: "Browser preview fallback",
        os: navigator.platform,
        arch: "web",
        sqlite_version: "n/a",
        database_path: "localStorage",
        database_bytes: stats.storage_bytes,
        mailbox: stats,
        cloud_configured: Boolean(window.__ELYRA_API_URL__),
      });
    }
    case "generate_encryption_key":
      return generateFallbackKey();
    case "encrypt_text":
      return fallbackEncrypt(args.key, args.plaintext);
    case "decrypt_text":
      return fallbackDecrypt(args.key, args.payload);
    case "fingerprint":
      return fallbackFingerprint(args.value);
    case "get_security_status":
      return Promise.resolve({
        status: "protected",
        checks_passed: 3,
        checks_total: 3,
        foreign_keys: true,
        journal_mode: "WAL",
        secure_delete: true,
        audit_events: 1,
      });
    case "list_audit_events":
      return Promise.resolve([
        { id: 1, created_at: sqlTime(), event_type: "security.initialized", outcome: "success", details: "Browser preview mode initialized" },
      ]);
    default:
      return Promise.resolve();
  }
}

export function isDesktopRuntime() {
  return Boolean(window.__TAURI__?.core?.invoke);
}

export function invoke(command, args = {}) {
  const tauri = window.__TAURI__;
  if (tauri?.core?.invoke) {
    return tauri.core.invoke(command, args);
  }
  return fallbackInvoke(command, args);
}
