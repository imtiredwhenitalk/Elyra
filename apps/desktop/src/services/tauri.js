const FALLBACK_STORAGE_PREFIX = "elyra-fallback";
const MAILBOX_STORE = "mailbox-v2";
const SETTINGS_STORE = "settings-v1";
const ACCOUNTS_STORE = "accounts-v1";
const SECURITY_STORE = "message-security-v1";
const CRYPTO_KEYS_STORE = "crypto-keys-v1";
const CRYPTO_CONTACTS_STORE = "crypto-contacts-v1";
const INTEGRATION_STATE_STORE = "integrations-v1";
const SENDER_RULES_STORE = "sender-rules-v1";
const REPUTATION_STORE = "reputation-v1";

const INTEGRATION_PRESETS = [
  {
    provider: "virustotal",
    label: "VirusTotal",
    description: "Checks links against 90+ security vendors and their URL reputation data.",
    docsUrl: "https://www.virustotal.com/gui/my-apikey",
    keyHint: "Paste the API key from your VirusTotal profile.",
  },
  {
    provider: "google_safe_browsing",
    label: "Google Safe Browsing",
    description: "Checks links against Google's constantly updated list of unsafe web resources.",
    docsUrl: "https://developers.google.com/safe-browsing/v4/get-started",
    keyHint: "Paste a Google Cloud API key with the Safe Browsing API enabled.",
  },
];

// The browser preview keeps integration secrets in memory only. The desktop
// build stores them in the OS keychain.
const integrationSecrets = new Map();

const PROVIDER_PRESETS = [
  { id: "gmail", label: "Gmail", imapHost: "imap.gmail.com", imapPort: 993, imapSecurity: "tls", smtpHost: "smtp.gmail.com", smtpPort: 465, smtpSecurity: "tls", hint: "Use a Google app password when 2-step verification is enabled." },
  { id: "outlook", label: "Outlook / Microsoft 365", imapHost: "outlook.office365.com", imapPort: 993, imapSecurity: "tls", smtpHost: "smtp.office365.com", smtpPort: 587, smtpSecurity: "starttls", hint: "Microsoft may require an app password or OAuth-enabled mailbox." },
  { id: "icloud", label: "iCloud Mail", imapHost: "imap.mail.me.com", imapPort: 993, imapSecurity: "tls", smtpHost: "smtp.mail.me.com", smtpPort: 587, smtpSecurity: "starttls", hint: "Generate an app-specific password in your Apple ID settings." },
  { id: "yahoo", label: "Yahoo Mail", imapHost: "imap.mail.yahoo.com", imapPort: 993, imapSecurity: "tls", smtpHost: "smtp.mail.yahoo.com", smtpPort: 465, smtpSecurity: "tls", hint: "Yahoo requires an app password for third-party clients." },
  { id: "fastmail", label: "Fastmail", imapHost: "imap.fastmail.com", imapPort: 993, imapSecurity: "tls", smtpHost: "smtp.fastmail.com", smtpPort: 465, smtpSecurity: "tls", hint: "Create an app password in Fastmail settings." },
  { id: "proton", label: "Proton Mail", imapHost: "127.0.0.1", imapPort: 1143, imapSecurity: "starttls", smtpHost: "127.0.0.1", smtpPort: 1025, smtpSecurity: "starttls", hint: "Proton requires Proton Mail Bridge running on this device." },
  { id: "custom", label: "Custom IMAP / SMTP", imapHost: "", imapPort: 993, imapSecurity: "tls", smtpHost: "", smtpPort: 465, smtpSecurity: "tls", hint: "Enter the server names, ports and security mode from your provider." },
];

function demoAccounts() {
  return [
    { id: 1, provider: "elyra", email: "alex@elyra.com", displayName: "Elyra local", username: "alex@elyra.com", imapHost: "", imapPort: 993, imapSecurity: "tls", smtpHost: "", smtpPort: 465, smtpSecurity: "tls", hasPassword: false, lastSyncAt: null, syncError: null },
    { id: 2, provider: "gmail", email: "alex.smith@gmail.com", displayName: "Personal", username: "alex.smith@gmail.com", imapHost: "", imapPort: 993, imapSecurity: "tls", smtpHost: "", smtpPort: 465, smtpSecurity: "tls", hasPassword: false, lastSyncAt: null, syncError: null },
    { id: 3, provider: "proton", email: "alex@proton.me", displayName: "Private", username: "alex@proton.me", imapHost: "", imapPort: 993, imapSecurity: "tls", smtpHost: "", smtpPort: 465, smtpSecurity: "tls", hasPassword: false, lastSyncAt: null, syncError: null },
  ];
}

function loadAccounts() {
  const stored = readStore(ACCOUNTS_STORE, null);
  if (!Array.isArray(stored) || !stored.length) {
    const accounts = demoAccounts();
    writeStore(ACCOUNTS_STORE, accounts);
    return accounts;
  }
  return stored;
}

function saveAccounts(accounts) {
  writeStore(ACCOUNTS_STORE, accounts);
}

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

const PRIORITY_URGENT_SIGNALS = ["urgent", "asap", "action required", "immediate action", "time sensitive", "final notice", "deadline today", "respond today"];
const PRIORITY_HIGH_SIGNALS = ["invoice", "payment", "contract", "proposal", "approval", "review", "deadline", "security alert", "sign-in", "password", "interview", "offer", "escalation", "needs your", "please confirm", "by tomorrow"];
const PRIORITY_LOW_SIGNALS = ["newsletter", "digest", "unsubscribe", "promotion", "weekly update", "automated", "notification", "no-reply", "noreply"];

function containsSignal(haystack, signals) {
  return signals.some((signal) => haystack.includes(signal));
}

function suggestPriority(sender, subject, body, tag) {
  const senderText = String(sender || "").toLowerCase();
  const subjectText = String(subject || "").toLowerCase();
  const bodyText = String(body || "").toLowerCase();
  const tagText = String(tag || "").toLowerCase();
  if (tagText === "spam" || tagText === "trash" || senderText.includes("no-reply") || senderText.includes("noreply") || containsSignal(subjectText, PRIORITY_LOW_SIGNALS)) {
    return { priority: "low", important: false, reason: "Looks like an automated or low-priority update." };
  }
  if (containsSignal(subjectText, PRIORITY_URGENT_SIGNALS)) {
    return { priority: "urgent", important: true, reason: "The subject asks for urgent action." };
  }
  const importantTag = tagText === "important" || tagText === "security";
  if (importantTag || containsSignal(subjectText, PRIORITY_HIGH_SIGNALS) || containsSignal(bodyText, ["by tomorrow", "needs your approval", "please review"])) {
    return { priority: "high", important: importantTag, reason: "Matches important or time-sensitive signals." };
  }
  return { priority: "normal", important: false, reason: "No strong priority signals detected." };
}

const FALLBACK_EXECUTABLE_EXTENSIONS = new Set(["exe", "scr", "com", "pif", "msi", "msp", "lnk", "dll", "sys", "cpl", "apk", "app", "deb", "rpm", "dmg", "pkg", "iso", "img", "vhd", "vhdx", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta", "bat", "cmd", "ps1", "sh", "jar"]);
const FALLBACK_SCRIPT_EXTENSIONS = new Set(["js", "jse", "vbs", "vbe", "wsf", "wsh", "hta", "bat", "cmd", "ps1", "sh", "py", "rb", "pl"]);
const FALLBACK_DISK_IMAGE_EXTENSIONS = new Set(["iso", "img", "vhd", "vhdx"]);
const FALLBACK_MACRO_EXTENSIONS = new Set(["docm", "dotm", "xlsm", "xltm", "xlam", "pptm", "potm", "ppam"]);
const FALLBACK_ARCHIVE_EXTENSIONS = new Set(["zip", "rar", "7z", "gz", "bz2", "xz", "tar", "cab", "arj", "lzh", "z", "tgz", "tbz2"]);
const FALLBACK_DISGUISED_EXTENSIONS = new Set(["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "rtf", "csv", "jpg", "jpeg", "png", "gif", "zip"]);

function fallbackAttachmentMeta(attachment) {
  const name = String(attachment.name || "attachment");
  const parts = name.split(".");
  const extension = parts.length > 1 ? parts[parts.length - 1].trim().toLowerCase() : "";
  const declared = String(attachment.mime_type || "application/octet-stream").toLowerCase();
  let kind = "other";
  let score = 0;
  const reasons = [];
  if (FALLBACK_DISK_IMAGE_EXTENSIONS.has(extension)) {
    kind = "disk-image";
    score += 30;
    reasons.push(`The attachment is a disk image (${extension}) and can carry hidden executables.`);
  } else if (FALLBACK_EXECUTABLE_EXTENSIONS.has(extension)) {
    kind = FALLBACK_SCRIPT_EXTENSIONS.has(extension) ? "script" : "executable";
    score += FALLBACK_SCRIPT_EXTENSIONS.has(extension) ? 35 : 45;
    reasons.push(FALLBACK_SCRIPT_EXTENSIONS.has(extension)
      ? `The attachment is a script (${extension}) and could run code when opened.`
      : `The attachment is an executable program (${extension}) and could harm this device.`);
  } else if (FALLBACK_MACRO_EXTENSIONS.has(extension)) {
    kind = "macro";
    score += 25;
    reasons.push(`The attachment is a macro-enabled Office document (${extension}).`);
  } else if (FALLBACK_ARCHIVE_EXTENSIONS.has(extension) || declared.includes("zip") || declared.includes("rar") || declared.includes("7z")) {
    kind = "archive";
    score += 25;
    reasons.push("The attachment is an archive. Archives can hide executables, so scan it before opening.");
  } else if (declared.startsWith("image/")) {
    kind = "image";
  } else if (declared === "application/pdf" || declared.startsWith("text/") || declared.includes("officedocument") || declared.includes("msword") || declared.includes("ms-excel") || declared.includes("ms-powerpoint")) {
    kind = "document";
  }
  if (parts.length >= 3 && FALLBACK_DISGUISED_EXTENSIONS.has(parts[parts.length - 2].toLowerCase()) && FALLBACK_EXECUTABLE_EXTENSIONS.has(extension)) {
    score += 35;
    reasons.push("The name uses a double extension to disguise an executable as a document.");
  }
  if (declared === "text/html") {
    score += 25;
    reasons.push("The attachment is a web page. HTML attachments are often used for phishing.");
  }
  score = Math.min(score, 100);
  const risk = score >= 60 ? "malicious" : score >= 25 ? "suspicious" : "safe";
  return {
    detected_mime: declared,
    kind,
    risk,
    score,
    reasons,
    sha256: null,
    content_available: false,
    scan_source: "local",
    scanned_at: sqlTime(),
    consent_required: false,
  };
}

function demoAttachmentsFor(row, index) {
  if (String(row[4]).toLowerCase() === "spam") {
    return [
      { id: index + 1, name: "claim-form.pdf.exe", mime_type: "application/pdf", size_bytes: 1_204_224 },
    ].map((attachment) => ({ ...attachment, ...fallbackAttachmentMeta(attachment) }));
  }
  if (!row[8]) return [];
  return [{ id: index + 1, name: "message-notes.pdf", mime_type: "application/pdf", size_bytes: 248_320 }]
    .map((attachment) => ({ ...attachment, ...fallbackAttachmentMeta(attachment) }));
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
  return rows.map((row, index) => {
    const suggestion = suggestPriority(row[0], row[2], row[9], row[5]);
    return {
      id: index + 1,
      sender: row[0],
      sender_address: demoAddress(row[0]),
      recipient: row[1],
      subject: row[2],
      received_at: sqlTime(row[3]),
      folder: row[4],
      tag: row[5],
      is_read: row[6],
      is_starred: row[7],
      is_important: suggestion.important,
      priority: suggestion.priority,
      priority_source: "auto",
      priority_reason: suggestion.reason,
      auth_results: String(row[4]).toLowerCase() === "spam"
        ? { spf: "fail", dkim: "fail", dmarc: "fail", source: "demo" }
        : { spf: "pass", dkim: "pass", dmarc: "pass", source: "demo" },
      has_attachments: row[8] || String(row[4]).toLowerCase() === "spam",
      body_text: row[9],
      preview: previewOf(row[9]),
      attachments: demoAttachmentsFor(row, index),
    };
  });
}

function loadMailbox() {
  let messages = readStore(MAILBOX_STORE, null);
  if (!Array.isArray(messages) || !messages.length) {
    messages = demoMessages();
    applySenderRulesTo(messages);
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
    is_important: Boolean(message.is_important),
    priority: message.priority || "normal",
    priority_source: message.priority_source || "auto",
    priority_reason: message.priority_reason || null,
    blocked_by: message.blocked_by || null,
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
    important: messages.filter((message) => message.is_important).length,
    high_priority: messages.filter((message) => ["high", "urgent"].includes(String(message.priority || "").toLowerCase())).length,
    attachments: messages.reduce((sum, message) => sum + (message.attachments?.length || 0), 0),
    trash: folderCounts.trash || 0,
    spam: folderCounts.spam || 0,
    drafts: folderCounts.drafts || 0,
    snoozed: folderCounts.snoozed || 0,
    accounts: loadAccounts().length,
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
    } else if (normalizedFilter === "important") {
      if (!message.is_important || ["trash", "spam"].includes(folder)) return false;
    } else if (normalizedFilter && normalizedFilter !== "all" && folder !== normalizedFilter) {
      return false;
    }
    if (normalizedQuick === "unread" && message.is_read) return false;
    if (normalizedQuick === "starred" && !message.is_starred) return false;
    if (normalizedQuick === "important" && !message.is_important) return false;
    if (normalizedQuick === "high" && !["high", "urgent"].includes(String(message.priority || "").toLowerCase())) return false;
    if (normalizedQuick === "urgent" && String(message.priority || "").toLowerCase() !== "urgent") return false;
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

function senderAddress(sender) {
  const text = String(sender || "").trim();
  const start = text.lastIndexOf("<");
  if (start !== -1) {
    const end = text.indexOf(">", start);
    if (end !== -1) {
      return text.slice(start + 1, end).trim().replace(/^"|"$/g, "").toLowerCase();
    }
  }
  return text.replace(/[<>",\s]+/g, "").toLowerCase();
}

function demoAddress(name) {
  const slug = String(name || "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ".")
    .replace(/^\.+|\.+$/g, "");
  return `${slug}@example.com`;
}

function senderDomain(address) {
  const at = String(address || "").lastIndexOf("@");
  if (at === -1) return "";
  return String(address).slice(at + 1).replace(/^\.+|\.+$/g, "").toLowerCase();
}

function normalizeSenderValue(scope, value) {
  const trimmed = String(value || "").trim();
  if (!trimmed) throw new Error("Enter an address or domain");
  if (scope === "domain") {
    const domain = trimmed.replace(/^@+/, "").replace(/^\.+|\.+$/g, "").toLowerCase();
    if (!domain || domain.includes("@") || /\s/.test(domain)) throw new Error("Enter a domain like example.com");
    if (!domain.includes(".")) throw new Error("Domains need at least one dot, like example.com");
    return domain;
  }
  const address = senderAddress(trimmed);
  if (!address.includes("@") || /\s/.test(address)) throw new Error("Enter a full email address like name@example.com");
  return address;
}

function senderRuleMatches(rule, address, domain) {
  if (rule.scope === "address") return rule.value === address;
  if (!domain) return false;
  return domain === rule.value || domain.endsWith(`.${rule.value}`);
}

function loadSenderRules() {
  const rules = readStore(SENDER_RULES_STORE, []);
  return Array.isArray(rules) ? rules : [];
}

function saveSenderRules(rules) {
  writeStore(SENDER_RULES_STORE, rules);
}

function applySenderRulesTo(messages) {
  const rules = loadSenderRules();
  const action = String(loadSettings().blocked_sender_action || "spam").toLowerCase() === "trash" ? "Trash" : "Spam";
  let blocked = 0;
  for (const message of messages) {
    const folder = String(message.folder || "").toLowerCase();
    const filteredFolder = folder === "spam" || folder === "trash";
    if (!["inbox", "snoozed", "spam", "trash"].includes(folder)) continue;
    if (filteredFolder && !message.blocked_by) continue;
    const address = String(message.sender_address || "").trim().toLowerCase() || senderAddress(message.sender);
    const domain = senderDomain(address);
    const allowed = rules.some((rule) => rule.kind === "allow" && senderRuleMatches(rule, address, domain));
    const blockRule = allowed ? null : rules.find((rule) => rule.kind === "block" && senderRuleMatches(rule, address, domain));
    if (blockRule) {
      message.blocked_by = `${blockRule.scope}:${blockRule.value}`;
      message.folder = action;
      blocked += 1;
    } else if (message.blocked_by) {
      message.blocked_by = null;
      if (filteredFolder) message.folder = "Inbox";
    }
  }
  return blocked;
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
    { version: "0.3.0", date: "2026-11-15", title: "Keychain + IMAP sync", detail: "Provider presets, account CRUD, connection tests and INBOX sync with passwords stored in the OS keychain.", status: "ready" },
    { version: "0.3.0", date: "2026-11-15", title: "First-run registration", detail: "Register once, choose your Elyra server URL, and send mail to any external address through the server relay.", status: "ready" },
    { version: "0.3.0", date: "2026-11-15", title: "Server deployment", detail: "Production Compose stack, optional Caddy HTTPS, health-checked deploy.sh and an opt-in GitHub Actions deploy workflow.", status: "ready" },
    { version: "0.2.0", date: "2026-11-14", title: "Rust mailbox core", detail: "Filtering, search, folders, drafts, bulk actions, export and mailbox statistics now run in the Rust core.", status: "ready" },
    { version: "0.2.0", date: "2026-11-14", title: "Presentable interface refresh", detail: "Consistent SVG icons, richer themes, accent colours, density controls, command palette and a calmer reading pane.", status: "ready" },
    { version: "0.2.0", date: "2026-11-14", title: "Installer and release pipeline", detail: "MSI release workflow, stable download names, a Rust installer launcher and a refreshed website.", status: "ready" },
    { version: "next", date: "planned", title: "Provider sync", detail: "Gmail and Outlook OAuth with PKCE, background sync and scheduled refresh are next.", status: "next" },
  ];
}

const PREVIEW_BRANDS = ["paypal", "microsoft", "google", "apple", "amazon", "github", "netflix", "dropbox", "docusign", "coinbase"];
const PREVIEW_URGENCY = ["urgent", "immediately", "verify your account", "account suspended", "final warning", "act now", "within 24 hours", "limited time", "click here", "confirm your identity", "unusual activity"];
const PREVIEW_CREDENTIALS = ["password", "one-time code", "otp", "social security", "credit card", "bank account", "wire transfer", "gift card", "seed phrase", "recovery code"];

function extractPreviewUrls(text) {
  const matches = String(text || "").match(/https?:\/\/[^\s<>"']+/g) || [];
  const seen = new Set();
  return matches
    .map((value) => value.replace(/[.,;:!?)\]}]+$/, ""))
    .filter((value) => {
      if (value.length < 10) return false;
      const key = value.toLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, 40);
}

function hostOfPreview(url) {
  try {
    return new URL(url).host.toLowerCase();
  } catch {
    return "";
  }
}

function fallbackIntegrationState() {
  return readStore(INTEGRATION_STATE_STORE, {});
}

function saveIntegrationState(state) {
  writeStore(INTEGRATION_STATE_STORE, state);
}

function fallbackIntegrationView(preset) {
  const state = fallbackIntegrationState()[preset.provider] || {};
  const configured = integrationSecrets.has(preset.provider);
  const status = !configured ? "not_configured" : state.status || "unverified";
  return {
    provider: preset.provider,
    label: preset.label,
    description: preset.description,
    docsUrl: preset.docsUrl,
    keyHint: preset.keyHint,
    enabled: configured && Boolean(state.enabled),
    configured,
    status,
    lastCheckedAt: state.lastCheckedAt || null,
    lastError: state.lastError || null,
  };
}

function fallbackListIntegrations() {
  return INTEGRATION_PRESETS.map(fallbackIntegrationView);
}

function fallbackConnectIntegration(provider, key) {
  const preset = INTEGRATION_PRESETS.find((item) => item.provider === provider);
  if (!preset) return Promise.reject(new Error("Unknown integration"));
  if (!key || key.trim().length < 8) return Promise.reject(new Error(`Enter the ${preset.label} API key before connecting the integration.`));
  integrationSecrets.set(provider, key.trim());
  const state = fallbackIntegrationState();
  state[provider] = { enabled: true, status: "connected", lastCheckedAt: sqlTime(), lastError: null };
  saveIntegrationState(state);
  return Promise.resolve(fallbackIntegrationView(preset));
}

function fallbackTestIntegration(provider) {
  const preset = INTEGRATION_PRESETS.find((item) => item.provider === provider);
  if (!preset) return Promise.reject(new Error("Unknown integration"));
  if (!integrationSecrets.has(provider)) return Promise.reject(new Error(`Add a ${preset.label} API key first.`));
  const state = fallbackIntegrationState();
  state[provider] = { ...(state[provider] || {}), enabled: true, status: "connected", lastCheckedAt: sqlTime(), lastError: null };
  saveIntegrationState(state);
  return Promise.resolve(fallbackIntegrationView(preset));
}

function fallbackSetIntegrationEnabled(provider, enabled) {
  const preset = INTEGRATION_PRESETS.find((item) => item.provider === provider);
  if (!preset) return Promise.reject(new Error("Unknown integration"));
  if (enabled && !integrationSecrets.has(provider)) return Promise.reject(new Error(`Add a ${preset.label} API key first.`));
  const state = fallbackIntegrationState();
  state[provider] = { ...(state[provider] || {}), enabled: Boolean(enabled), status: enabled ? "connected" : "disabled", lastCheckedAt: sqlTime(), lastError: null };
  saveIntegrationState(state);
  return Promise.resolve(fallbackIntegrationView(preset));
}

function fallbackRemoveIntegration(provider) {
  const preset = INTEGRATION_PRESETS.find((item) => item.provider === provider);
  if (!preset) return Promise.reject(new Error("Unknown integration"));
  integrationSecrets.delete(provider);
  const state = fallbackIntegrationState();
  state[provider] = { enabled: false, status: "not_configured", lastCheckedAt: null, lastError: null };
  saveIntegrationState(state);
  return Promise.resolve(fallbackIntegrationView(preset));
}

function fallbackSecurityState() {
  return readStore(SECURITY_STORE, {});
}

function fallbackAnalyzeMessage(id, rescan = false) {
  const messages = loadMailbox();
  const message = messages.find((item) => Number(item.id) === Number(id));
  if (!message) return Promise.reject(new Error("Message not found"));
  const state = fallbackSecurityState();
  const key = String(id);
  if (state[key] && !rescan) return Promise.resolve(state[key]);

  const subject = String(message.subject || "");
  const body = String(message.body_text || "");
  const haystack = `${subject}\n${body}`.toLowerCase();
  const reasons = [];
  let score = 0;

  const sender = String(message.sender || "");
  const addressMatch = sender.match(/([a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,})/i);
  const address = addressMatch ? addressMatch[1].toLowerCase() : "";
  const domain = address.includes("@") ? address.split("@").pop() : "";
  const display = sender.toLowerCase();
  const local = address.split("@")[0] || "";
  const official = {
    paypal: ["paypal.com", "paypal.me"],
    microsoft: ["microsoft.com", "outlook.com", "live.com", "office.com"],
    google: ["google.com", "gmail.com"],
    apple: ["apple.com", "icloud.com", "me.com"],
    amazon: ["amazon.com", "amazon.co.uk"],
    github: ["github.com"],
    netflix: ["netflix.com"],
    dropbox: ["dropbox.com"],
    docusign: ["docusign.com", "docusign.net"],
    coinbase: ["coinbase.com"],
  };
  for (const brand of PREVIEW_BRANDS) {
    if (!domain) break;
    const trusted = official[brand] || [];
    if (trusted.some((value) => domain === value || domain.endsWith(`.${value}`))) continue;
    const normalized = domain.replace(/[^a-z0-9]/g, "").replace(/0/g, "o").replace(/1/g, "l").replace(/3/g, "e").replace(/5/g, "s");
    if (display.includes(brand) || local.includes(brand) || normalized.includes(brand)) {
      score += 28;
      reasons.push(`The sender claims to be ${brand}, but the address ${address} is not on an official ${brand} domain.`);
      break;
    }
  }
  if (domain.startsWith("xn--") || domain.includes(".xn--")) {
    score += 20;
    reasons.push(`The sender domain ${domain} uses punycode, which is often used to imitate a trusted brand.`);
  }
  const urgency = PREVIEW_URGENCY.filter((phrase) => haystack.includes(phrase)).length;
  if (urgency >= 2) {
    score += 18;
    reasons.push("The message uses urgent or threatening language common in phishing campaigns.");
  }
  if (PREVIEW_CREDENTIALS.some((phrase) => haystack.includes(phrase))) {
    score += 10;
    reasons.push("The message asks for credentials or payment details. Legitimate providers do not request these by email.");
  }
  const dangerous = (message.attachments || []).some((attachment) => /\.(exe|scr|js|jse|vbs|vbe|bat|cmd|com|pif|msi|iso|lnk|hta|jar|ps1)$/i.test(attachment.name || ""));
  if (dangerous) {
    score += 30;
    reasons.push("The message includes an executable attachment.");
  }
  const urls = extractPreviewUrls(body).map((url) => ({
    url,
    host: hostOfPreview(url),
    verdict: "unscanned",
    score: 0,
    providers: [],
    cached: false,
    checkedAt: sqlTime(),
  }));
  const risk = score >= 60 ? "malicious" : score >= 25 ? "suspicious" : "safe";
  const analysis = {
    emailId: Number(id),
    risk,
    score,
    reasons,
    urls,
    externalScanned: false,
    falsePositive: false,
    falsePositiveNote: null,
    analyzedAt: sqlTime(),
  };
  state[key] = analysis;
  writeStore(SECURITY_STORE, state);
  return Promise.resolve(analysis);
}

function fallbackListSecurityAlerts(limit = 25) {
  const state = fallbackSecurityState();
  const messages = loadMailbox();
  return Object.values(state)
    .filter((analysis) => analysis.risk === "suspicious" || analysis.risk === "malicious")
    .map((analysis) => {
      const message = messages.find((item) => Number(item.id) === Number(analysis.emailId));
      return message
        ? {
            emailId: analysis.emailId,
            risk: analysis.risk,
            score: analysis.score,
            reasons: analysis.reasons,
            falsePositive: Boolean(analysis.falsePositive),
            analyzedAt: analysis.analyzedAt,
            sender: message.sender,
            subject: message.subject,
          }
        : null;
    })
    .filter(Boolean)
    .sort((left, right) => right.score - left.score)
    .slice(0, limit);
}

function fallbackShieldSummary() {
  const analyses = Object.values(fallbackSecurityState());
  const providers = fallbackListIntegrations().filter((item) => item.enabled).length;
  return {
    analyzed: analyses.length,
    safe: analyses.filter((item) => item.risk === "safe").length,
    suspicious: analyses.filter((item) => item.risk === "suspicious").length,
    malicious: analyses.filter((item) => item.risk === "malicious").length,
    falsePositives: analyses.filter((item) => item.falsePositive).length,
    providersConnected: providers,
    externalScanning: readStore(SETTINGS_STORE, {}).external_scanning !== "false",
  };
}

function fallbackReportFalsePositive(id, note) {
  const state = fallbackSecurityState();
  const analysis = state[String(id)];
  if (!analysis) return Promise.reject(new Error("Analyze this message before reporting it."));
  analysis.falsePositive = true;
  analysis.falsePositiveNote = note || null;
  writeStore(SECURITY_STORE, state);
  return Promise.resolve(analysis);
}

function fallbackAuthResults(message) {
  const value = message.auth_results;
  if (!value) return {};
  if (typeof value === "string") {
    try {
      return JSON.parse(value) || {};
    } catch {
      return {};
    }
  }
  return value;
}

function fallbackHasAuth(auth) {
  return Boolean(auth.spf || auth.dkim || auth.dmarc);
}

function fallbackAuthScore(auth, reasons) {
  let score = 0;
  const spf = String(auth.spf || "").toLowerCase();
  const dkim = String(auth.dkim || "").toLowerCase();
  const dmarc = String(auth.dmarc || "").toLowerCase();
  if (spf === "fail") {
    score += 18;
    reasons.push("SPF failed: the sending server is not authorized for this domain.");
  } else if (["softfail", "neutral", "permerror", "temperror"].includes(spf)) {
    score += 8;
    reasons.push(`SPF did not pass (${spf}), so the sender could not be fully verified.`);
  }
  if (dkim === "fail") {
    score += 12;
    reasons.push("DKIM failed: the message signature did not verify.");
  }
  if (dmarc === "fail") {
    score += 22;
    reasons.push("DMARC failed: the message does not align with the domain's published policy.");
  } else if (dmarc === "none") {
    reasons.push("The domain has no DMARC policy, so spoofing protection is limited.");
  } else if (["softfail", "permerror", "temperror"].includes(dmarc)) {
    score += 8;
    reasons.push(`DMARC did not pass (${dmarc}).`);
  }
  return score;
}

function fallbackBrandImpersonation(display, address) {
  const domain = senderDomain(address);
  if (!domain) return null;
  if (domain.startsWith("xn--") || domain.includes(".xn--")) {
    return `The sender domain ${domain} uses punycode, which is often used to imitate a trusted brand.`;
  }
  const official = {
    paypal: ["paypal.com", "paypal.me"],
    microsoft: ["microsoft.com", "outlook.com", "live.com", "office.com"],
    google: ["google.com", "gmail.com"],
    apple: ["apple.com", "icloud.com", "me.com"],
    amazon: ["amazon.com", "amazon.co.uk"],
    github: ["github.com"],
    netflix: ["netflix.com"],
    dropbox: ["dropbox.com"],
    docusign: ["docusign.com", "docusign.net"],
    coinbase: ["coinbase.com"],
  };
  const text = String(display || "").toLowerCase();
  const local = address.split("@")[0] || "";
  const normalized = domain.replace(/[^a-z0-9]/g, "").replace(/0/g, "o").replace(/1/g, "l").replace(/3/g, "e").replace(/5/g, "s");
  for (const brand of PREVIEW_BRANDS) {
    const trusted = official[brand] || [];
    if (trusted.some((value) => domain === value || domain.endsWith(`.${value}`))) continue;
    if (text.includes(brand) || local.includes(brand) || normalized.includes(brand)) {
      return `The sender claims to be ${brand}, but the address ${address} is not on an official ${brand} domain.`;
    }
  }
  return null;
}

function fallbackSenderReputation(id) {
  const messages = loadMailbox();
  const message = messages.find((item) => Number(item.id) === Number(id));
  if (!message) return Promise.reject(new Error("Message not found"));
  const address = String(message.sender_address || "").trim().toLowerCase() || senderAddress(message.sender);
  const domain = senderDomain(address);
  const rules = loadSenderRules();
  const allowlisted = rules.some((rule) => rule.kind === "allow" && senderRuleMatches(rule, address, domain));
  const blockRule = allowlisted ? null : rules.find((rule) => rule.kind === "block" && senderRuleMatches(rule, address, domain));
  const auth = fallbackAuthResults(message);
  const reasons = [];
  let score = 0;
  let trusted = false;

  if (allowlisted) {
    trusted = true;
    reasons.push("You allowlisted this sender, so it is treated as trusted.");
  } else {
    if (blockRule) {
      score += 20;
      reasons.push(`You blocked this sender ${blockRule.scope} (${blockRule.value}).`);
    }
    const impersonation = fallbackBrandImpersonation(message.sender, address);
    if (impersonation) {
      score += 28;
      reasons.push(impersonation);
    }
    score += fallbackAuthScore(auth, reasons);
    const security = fallbackSecurityState()[String(id)] || {};
    for (const scan of security.urls || []) {
      if (!scan.host || !domain) continue;
      const host = String(scan.host).toLowerCase();
      if (host === domain || host.endsWith(`.${domain}`)) {
        if (scan.verdict === "malicious") {
          score += 60;
          reasons.push(`Threat intelligence flagged a link on ${domain} as malicious.`);
        } else if (scan.verdict === "suspicious") {
          score += 30;
          reasons.push(`Threat intelligence marked a link on ${domain} as suspicious.`);
        }
      }
    }
    if (!reasons.length) {
      reasons.push(fallbackHasAuth(auth)
        ? "SPF, DKIM and DMARC did not report any problems."
        : "No SPF, DKIM, or DMARC results were available for this domain.");
    }
  }

  const classification = trusted ? "trusted" : score >= 60 ? "malicious" : score >= 25 ? "suspicious" : "neutral";
  const store = readStore(REPUTATION_STORE, {});
  const existing = domain ? store[domain] || {} : {};
  let disputed = Boolean(existing.disputed);
  let dismissedScore = Number(existing.dismissedScore) || 0;
  const feedbackNote = existing.feedbackNote || null;
  let finalClass = classification;
  let finalScore = score;
  let finalReasons = reasons;

  if (disputed) {
    if (score <= dismissedScore) {
      finalClass = "neutral";
      finalScore = dismissedScore;
      finalReasons = ["You reported this assessment as incorrect. Elyra is showing it as Neutral until new intelligence arrives."];
    } else {
      disputed = false;
      dismissedScore = 0;
      finalReasons = [...reasons, "New intelligence changed this assessment after your report."];
    }
  }

  const updatedAt = sqlTime();
  if (domain) {
    store[domain] = {
      classification: finalClass,
      score: finalScore,
      reasons: finalReasons,
      disputed,
      dismissedScore,
      feedbackNote,
      updatedAt,
    };
    writeStore(REPUTATION_STORE, store);
  }
  return Promise.resolve({
    emailId: Number(id),
    domain,
    address: address || null,
    classification: finalClass,
    score: finalScore,
    reasons: finalReasons,
    auth,
    allowlisted,
    blocklisted: Boolean(blockRule),
    disputed,
    feedbackNote,
    updatedAt,
  });
}

function fallbackRefreshDomainReputation(domain) {
  const target = String(domain || "").trim().replace(/^@+/, "").toLowerCase();
  const messages = loadMailbox();
  const selected = messages.filter((message) => {
    if (!target) return true;
    const address = String(message.sender_address || "").trim().toLowerCase() || senderAddress(message.sender);
    return senderDomain(address) === target;
  });
  return Promise.all(selected.map((message) => fallbackSenderReputation(message.id))).then((items) => {
    return new Set(items.map((item) => item.domain).filter(Boolean)).size;
  });
}

function fallbackReportReputationIssue(id, note) {
  return fallbackSenderReputation(id).then((reputation) => {
    if (!reputation.domain) return Promise.reject(new Error("This message has no sender domain to assess."));
    const store = readStore(REPUTATION_STORE, {});
    const existing = store[reputation.domain] || {};
    store[reputation.domain] = {
      ...existing,
      classification: "neutral",
      score: reputation.score,
      disputed: true,
      dismissedScore: reputation.score,
      feedbackNote: String(note || "").trim() || null,
      updatedAt: sqlTime(),
    };
    writeStore(REPUTATION_STORE, store);
    return fallbackSenderReputation(id);
  });
}

function fallbackKeys() {
  const keys = readStore(CRYPTO_KEYS_STORE, []);
  return Array.isArray(keys) ? keys : [];
}

function fallbackContacts() {
  const contacts = readStore(CRYPTO_CONTACTS_STORE, []);
  return Array.isArray(contacts) ? contacts : [];
}

function nextLocalId(items) {
  return items.reduce((max, item) => Math.max(max, Number(item.id) || 0), 0) + 1;
}

async function fallbackNewKey(label, keys) {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  const publicKey = toBase64([...bytes]);
  return {
    id: nextLocalId(keys),
    label,
    algorithm: "Ed25519",
    publicKey,
    fingerprint: await fallbackFingerprint(publicKey),
    status: "active",
    createdAt: sqlTime(),
    rotatedAt: null,
    revokedAt: null,
    hasPrivateKey: true,
  };
}

async function fallbackGenerateKey(label) {
  const keys = fallbackKeys();
  if (keys.some((key) => key.status === "active")) throw new Error("An active identity key already exists. Rotate it instead.");
  const key = await fallbackNewKey(String(label || "").trim() || "Primary identity", keys);
  keys.push(key);
  writeStore(CRYPTO_KEYS_STORE, keys);
  return key;
}

async function fallbackRotateKey(id, label) {
  const keys = fallbackKeys();
  const existing = keys.find((key) => Number(key.id) === Number(id));
  if (!existing) throw new Error("Identity key not found.");
  if (existing.status === "revoked") throw new Error("A revoked key cannot be rotated. Generate a new identity key.");
  const created = await fallbackNewKey(String(label || "").trim() || existing.label, keys);
  existing.status = "retired";
  existing.rotatedAt = sqlTime();
  keys.push(created);
  writeStore(CRYPTO_KEYS_STORE, keys);
  return created;
}

function fallbackRevokeKey(id) {
  const keys = fallbackKeys();
  const existing = keys.find((key) => Number(key.id) === Number(id));
  if (!existing) return Promise.reject(new Error("Identity key not found."));
  existing.status = "revoked";
  existing.revokedAt = sqlTime();
  existing.hasPrivateKey = false;
  writeStore(CRYPTO_KEYS_STORE, keys);
  return Promise.resolve(existing);
}

function fallbackExportKey(id) {
  const existing = fallbackKeys().find((key) => Number(key.id) === Number(id));
  if (!existing) return Promise.reject(new Error("Identity key not found."));
  return Promise.resolve(`-----BEGIN ELYRA PUBLIC KEY-----\n${existing.publicKey}\n-----END ELYRA PUBLIC KEY-----`);
}

async function fallbackImportContact(email, label, publicKey) {
  const address = String(email || "").trim().toLowerCase();
  if (!address.includes("@")) throw new Error("Enter the contact's email address.");
  const encoded = String(publicKey || "").replace(/-----[^-]+-----/g, "").replace(/\s+/g, "");
  const bytes = fromBase64(encoded);
  if (bytes.length !== 32) throw new Error("An Ed25519 public key must contain 32 bytes.");
  const contacts = fallbackContacts();
  const existing = contacts.find((contact) => contact.email === address);
  const contact = {
    id: existing?.id || nextLocalId(contacts),
    email: address,
    label: String(label || "").trim() || address,
    publicKey: encoded,
    fingerprint: await fallbackFingerprint(encoded),
    importedAt: sqlTime(),
  };
  writeStore(CRYPTO_CONTACTS_STORE, existing ? contacts.map((item) => (item.id === existing.id ? contact : item)) : [...contacts, contact]);
  return contact;
}

function fallbackDeleteContact(id) {
  const contacts = fallbackContacts();
  writeStore(CRYPTO_CONTACTS_STORE, contacts.filter((contact) => Number(contact.id) !== Number(id)));
  return Promise.resolve();
}

function fallbackAttachmentScan(id) {
  const messages = loadMailbox();
  for (const message of messages) {
    const attachment = (message.attachments || []).find((item) => Number(item.id) === Number(id));
    if (!attachment) continue;
    Object.assign(attachment, fallbackAttachmentMeta(attachment));
    saveMailbox(messages);
    return Promise.resolve({
      attachment_id: attachment.id,
      email_id: message.id,
      name: attachment.name,
      mime_type: attachment.mime_type,
      size_bytes: attachment.size_bytes,
      ...fallbackAttachmentMeta(attachment),
    });
  }
  return Promise.reject(new Error("Attachment not found"));
}

function fallbackScanAllAttachments() {
  const messages = loadMailbox();
  let scanned = 0;
  for (const message of messages) {
    for (const attachment of message.attachments || []) {
      Object.assign(attachment, fallbackAttachmentMeta(attachment));
      scanned += 1;
    }
  }
  saveMailbox(messages);
  return Promise.resolve(scanned);
}

function fallbackPreference(value, fallback) {
  if (value === undefined || value === null) return fallback;
  return value === true || String(value) === "true";
}

function fallbackUpdatePreferences() {
  const settings = loadSettings();
  return {
    autoCheck: fallbackPreference(settings.updatesAutoCheck, true),
    notifyAvailable: fallbackPreference(settings.updatesNotifyAvailable, true),
    lastCheckedAt: settings.updatesLastCheckedAt || null,
    lastResult: settings.updatesLastResult || null,
  };
}

function fallbackCheckForUpdates() {
  const checkedAt = new Date().toISOString();
  const status = {
    status: "upToDate",
    available: false,
    currentVersion: "0.3.0",
    latestVersion: null,
    publishedAt: null,
    notes: null,
    target: "browser-preview",
    checkedAt,
    message: "Browser preview does not download desktop updates. Build the Windows app to check the signed release feed.",
  };
  saveSettings({ updatesLastCheckedAt: checkedAt, updatesLastResult: status });
  return Promise.resolve(status);
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
    case "scan_attachment":
      return fallbackAttachmentScan(args.id);
    case "scan_all_attachments":
      return fallbackScanAllAttachments();
    case "scan_attachment_external":
      return Promise.reject(new Error("VirusTotal file uploads are only available in the desktop build."));
    case "get_attachment_content":
      return Promise.reject(new Error("Attachment content is only stored in the desktop build."));
    case "update_message": {
      const messages = loadMailbox();
      const message = messages.find((item) => Number(item.id) === Number(args.id));
      if (!message) return Promise.reject(new Error("Message not found"));
      if (args.isRead !== undefined) message.is_read = Boolean(args.isRead);
      if (args.isStarred !== undefined) message.is_starred = Boolean(args.isStarred);
      if (args.isImportant !== undefined) message.is_important = Boolean(args.isImportant);
      if (args.priority !== undefined) {
        message.priority = String(args.priority || "normal").toLowerCase();
        message.priority_source = "manual";
        message.priority_reason = "Set manually.";
      }
      if (args.folder) {
        message.folder = args.folder;
        message.blocked_by = null;
      }
      saveMailbox(messages);
      return Promise.resolve();
    }
    case "refresh_priority_suggestions": {
      const messages = loadMailbox();
      const settings = loadSettings();
      const automatic = String(settings.auto_priority ?? "true") !== "false";
      const fallbackPriority = settings.priority_default || "normal";
      let updated = 0;
      for (const message of messages) {
        if (String(message.priority_source || "auto") !== "auto") continue;
        const suggestion = suggestPriority(message.sender, message.subject, message.body_text, message.tag);
        message.priority = automatic ? suggestion.priority : fallbackPriority;
        message.priority_source = "auto";
        message.priority_reason = automatic ? suggestion.reason : null;
        updated += 1;
      }
      saveMailbox(messages);
      return Promise.resolve(updated);
    }
    case "list_sender_rules":
      return Promise.resolve(loadSenderRules());
    case "save_sender_rule": {
      const kind = String(args.kind || "").toLowerCase();
      const scope = String(args.scope || "").toLowerCase();
      if (!["block", "allow"].includes(kind)) return Promise.reject(new Error("Rule type must be block or allow"));
      if (!["address", "domain"].includes(scope)) return Promise.reject(new Error("Rule scope must be address or domain"));
      let value;
      try {
        value = normalizeSenderValue(scope, args.value);
      } catch (error) {
        return Promise.reject(error);
      }
      const note = String(args.note || "").trim() || null;
      const rules = loadSenderRules();
      const existing = rules.find((rule) => rule.kind === kind && rule.scope === scope && rule.value === value);
      let rule;
      if (existing) {
        existing.note = note;
        rule = existing;
      } else {
        rule = { id: nextLocalId(rules), kind, scope, value, note, created_at: sqlTime() };
        rules.push(rule);
      }
      saveSenderRules(rules);
      const messages = loadMailbox();
      applySenderRulesTo(messages);
      saveMailbox(messages);
      return Promise.resolve(rule);
    }
    case "delete_sender_rule": {
      const rules = loadSenderRules();
      const remaining = rules.filter((rule) => Number(rule.id) !== Number(args.id));
      saveSenderRules(remaining);
      const messages = loadMailbox();
      applySenderRulesTo(messages);
      saveMailbox(messages);
      return Promise.resolve(remaining.length !== rules.length);
    }
    case "apply_sender_rules": {
      const messages = loadMailbox();
      const blocked = applySenderRulesTo(messages);
      saveMailbox(messages);
      return Promise.resolve(blocked);
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
          if (action === "important") message.is_important = true;
          if (action === "unimportant") message.is_important = false;
          if (action.startsWith("priority-")) {
            message.priority = action.slice("priority-".length);
            message.priority_source = "manual";
            message.priority_reason = "Set manually.";
          }
          if (action === "trash") message.folder = "Trash";
          if (action === "spam") message.folder = "Spam";
          if (action === "archive") message.folder = "Archive";
          if (action === "inbox" || action === "restore") message.folder = "Inbox";
          if (action === "snooze") {
            message.folder = "Snoozed";
            message.snoozed_until = args.until || sqlTime(-1_440);
          }
          if (["trash", "spam", "archive", "inbox", "restore", "snooze"].includes(action)) {
            message.blocked_by = null;
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
          is_important: false,
          priority: "normal",
          priority_source: "auto",
          priority_reason: null,
          blocked_by: null,
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
        version: "0.3.0",
        exported_at: sqlTime(),
        messages: loadMailbox(),
        settings: loadSettings(),
      });
    case "send_email": {
      const messages = loadMailbox();
      const account = loadAccounts().find((item) => Number(item.id) === Number(args.accountId));
      messages.unshift({
        id: nextId(messages),
        sender: account?.displayName || "Alex Smith",
        recipient: args.to,
        subject: args.subject,
        body_text: args.body,
        preview: previewOf(args.body),
        folder: "Sent",
        tag: "Sent",
        is_read: true,
        is_starred: false,
        is_important: false,
        priority: "normal",
        priority_source: "auto",
        priority_reason: null,
        blocked_by: null,
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
    case "provider_presets":
      return Promise.resolve(PROVIDER_PRESETS);
    case "list_email_accounts":
      return Promise.resolve(loadAccounts());
    case "save_email_account": {
      const accounts = loadAccounts();
      const input = args.account || {};
      const existing = accounts.find((item) => Number(item.id) === Number(input.id));
      const account = {
        id: existing?.id || nextId(accounts),
        provider: input.provider || "custom",
        email: String(input.email || "").trim().toLowerCase(),
        displayName: input.displayName || input.email || "Email account",
        username: input.username || input.email || "",
        imapHost: input.imapHost || "",
        imapPort: Number(input.imapPort) || 993,
        imapSecurity: input.imapSecurity || "tls",
        smtpHost: input.smtpHost || "",
        smtpPort: Number(input.smtpPort) || 465,
        smtpSecurity: input.smtpSecurity || "tls",
        hasPassword: Boolean(input.password) || Boolean(existing?.hasPassword),
        lastSyncAt: existing?.lastSyncAt || null,
        syncError: null,
      };
      saveAccounts(existing
        ? accounts.map((item) => (item.id === existing.id ? account : item))
        : [...accounts, account]);
      return Promise.resolve(account);
    }
    case "delete_email_account":
      saveAccounts(loadAccounts().filter((item) => Number(item.id) !== Number(args.id)));
      return Promise.resolve();
    case "test_email_account":
    case "test_email_settings":
      return Promise.resolve({
        imapOk: false,
        smtpOk: false,
        detail: "Connection tests run in the desktop build with the Rust IMAP and SMTP engine.",
      });
    case "sync_email_account":
    case "sync_all_accounts":
      return Promise.resolve({ accounts: [], imported: 0 });
    case "get_changelog":
      return Promise.resolve(changelog());
    case "get_update_preferences":
      return Promise.resolve(fallbackUpdatePreferences());
    case "set_update_preferences": {
      const current = fallbackUpdatePreferences();
      const next = {
        autoCheck: args.autoCheck === undefined ? current.autoCheck : Boolean(args.autoCheck),
        notifyAvailable: args.notifyAvailable === undefined ? current.notifyAvailable : Boolean(args.notifyAvailable),
      };
      saveSettings({
        updatesAutoCheck: String(next.autoCheck),
        updatesNotifyAvailable: String(next.notifyAvailable),
      });
      return Promise.resolve({ ...current, ...next });
    }
    case "check_for_updates":
      return fallbackCheckForUpdates();
    case "download_and_install_update":
      return Promise.reject(new Error("Updates can only be installed from the Windows desktop build."));
    case "get_app_diagnostics": {
      const stats = statsOf(loadMailbox());
      return Promise.resolve({
        app_version: "0.3.0",
        engine: "Browser preview fallback",
        os: navigator.platform,
        arch: "web",
        sqlite_version: "n/a",
        database_path: "localStorage",
        database_bytes: stats.storage_bytes,
        mailbox: stats,
        cloud_configured: Boolean(window.__ELYRA_API_URL__),
        keychain_available: false,
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
    case "list_integrations":
      return Promise.resolve(fallbackListIntegrations());
    case "connect_integration":
      return fallbackConnectIntegration(args.provider, args.key);
    case "test_integration":
      return fallbackTestIntegration(args.provider);
    case "set_integration_enabled":
      return fallbackSetIntegrationEnabled(args.provider, args.enabled);
    case "remove_integration_key":
      return fallbackRemoveIntegration(args.provider);
    case "scan_links": {
      const urls = extractPreviewUrls(args.text).map((url) => ({
        url,
        host: hostOfPreview(url),
        verdict: "unscanned",
        score: 0,
        providers: [],
        cached: false,
        checkedAt: sqlTime(),
      }));
      return Promise.resolve({
        urls,
        scanned: false,
        disabled: readStore(SETTINGS_STORE, {}).external_scanning === "false",
        providers: [],
        note: "External providers are only called by the desktop build.",
      });
    }
    case "analyze_message":
      return fallbackAnalyzeMessage(args.id, Boolean(args.rescan));
    case "get_message_security":
      return fallbackAnalyzeMessage(args.id, false);
    case "list_security_alerts":
      return Promise.resolve(fallbackListSecurityAlerts(args.limit || 25));
    case "get_shield_summary":
      return Promise.resolve(fallbackShieldSummary());
    case "report_false_positive":
      return fallbackReportFalsePositive(args.id, args.note);
    case "get_sender_reputation":
      return fallbackSenderReputation(args.id);
    case "refresh_domain_reputation":
      return fallbackRefreshDomainReputation(args.domain);
    case "report_reputation_issue":
      return fallbackReportReputationIssue(args.id, args.note);
    case "list_crypto_keys":
      return Promise.resolve(fallbackKeys());
    case "generate_crypto_key":
      return fallbackGenerateKey(args.label);
    case "rotate_crypto_key":
      return fallbackRotateKey(args.id, args.label);
    case "revoke_crypto_key":
      return fallbackRevokeKey(args.id);
    case "export_public_key":
      return fallbackExportKey(args.id);
    case "import_public_key":
      return fallbackImportContact(args.email, args.label, args.publicKey);
    case "list_crypto_contacts":
      return Promise.resolve(fallbackContacts());
    case "delete_crypto_contact":
      return fallbackDeleteContact(args.id);
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

export function listen(event, handler) {
  const eventApi = window.__TAURI__?.event;
  if (eventApi?.listen) {
    return eventApi.listen(event, handler);
  }
  return Promise.resolve(() => {});
}
