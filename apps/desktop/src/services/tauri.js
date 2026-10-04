const FALLBACK_STORAGE_PREFIX = "elyra-fallback";

function readFallbackStore(key, fallback = []) {
  try {
    const raw = window.localStorage.getItem(`${FALLBACK_STORAGE_PREFIX}:${key}`);
    return raw ? JSON.parse(raw) : fallback;
  } catch {
    return fallback;
  }
}

function writeFallbackStore(key, value) {
  try {
    window.localStorage.setItem(`${FALLBACK_STORAGE_PREFIX}:${key}`, JSON.stringify(value));
  } catch {
    // Ignore storage write failures in browser preview mode.
  }
}

function toBase64(bytes) {
  if (typeof window !== "undefined" && window.btoa) {
    let binary = "";
    bytes.forEach((byte) => {
      binary += String.fromCharCode(byte);
    });
    return window.btoa(binary);
  }

  return Buffer.from(bytes).toString("base64");
}

function fromBase64(value) {
  if (typeof window !== "undefined" && window.atob) {
    const binary = window.atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    return bytes;
  }

  return Uint8Array.from(Buffer.from(value, "base64"));
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
  if (decoded.length <= 12) {
    throw new Error("Encrypted payload is incomplete");
  }
  const iv = decoded.slice(0, 12);
  const ciphertext = decoded.slice(12);
  const cryptoKey = await crypto.subtle.importKey("raw", keyBytes, "AES-GCM", false, ["decrypt"]);
  const plainText = await crypto.subtle.decrypt({ name: "AES-GCM", iv }, cryptoKey, ciphertext);
  return new TextDecoder().decode(plainText);
}

async function fallbackFingerprint(value) {
  const input = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest("SHA-256", input);
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function fallbackInvoke(command, args = {}) {
  switch (command) {
    case "initialize_local_storage":
      return Promise.resolve();
    case "list_cached_emails":
      return Promise.resolve(readFallbackStore("emails", []));
    case "cache_email": {
      const emails = readFallbackStore("emails", []);
      const next = [{
        id: Date.now(),
        provider_id: args.provider_id || `preview-${Date.now()}`,
        sender: args.sender || "Local preview",
        subject: args.subject || "Untitled message",
        body_text: args.body_text || "",
        folder: args.folder || "Inbox",
        is_read: Boolean(args.is_read),
        is_starred: Boolean(args.is_starred),
        ...args,
      }, ...emails.filter((email) => email.provider_id !== args.provider_id)];
      writeFallbackStore("emails", next);
      return Promise.resolve();
    }
    case "set_local_setting": {
      const settings = readFallbackStore("settings", {});
      settings[args.key] = args.value;
      writeFallbackStore("settings", settings);
      return Promise.resolve();
    }
    case "send_email": {
      const sent = readFallbackStore("sent-emails", []);
      sent.unshift({
        to: args.to,
        subject: args.subject,
        body: args.body,
        sentAt: new Date().toISOString(),
      });
      writeFallbackStore("sent-emails", sent);
      return Promise.resolve();
    }
    case "list_sent_emails":
      return Promise.resolve(readFallbackStore("sent-emails", []));
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
        { id: 1, created_at: new Date().toISOString(), event_type: "security.initialized", outcome: "success", details: "Browser preview mode initialized" },
      ]);
    default:
      return Promise.resolve();
  }
}

export function invoke(command, args = {}) {
  const tauri = window.__TAURI__;
  if (tauri?.core?.invoke) {
    return tauri.core.invoke(command, args);
  }
  return fallbackInvoke(command, args);
}
