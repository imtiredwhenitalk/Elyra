import { invoke, isDesktopRuntime } from "./tauri.js";

const sessionKey = "elyra-session";
const usersKey = "elyra-preview-users";
const notificationsKey = "elyra-preview-notifications";
const apiBase = (window.__ELYRA_API_URL__ || "http://127.0.0.1:8787").replace(/\/$/, "");

function readJson(key, fallback) {
  try {
    return JSON.parse(localStorage.getItem(key) || JSON.stringify(fallback));
  } catch {
    return fallback;
  }
}

function writeJson(key, value) {
  localStorage.setItem(key, JSON.stringify(value));
}

function browserPreview() {
  return !isDesktopRuntime() && !window.__ELYRA_API_URL__;
}

async function passwordDigest(password) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(password));
  return Array.from(new Uint8Array(digest)).map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function request(path, options = {}) {
  const session = getSession();
  const response = await fetch(`${apiBase}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(session?.accessToken ? { Authorization: `Bearer ${session.accessToken}` } : {}),
      ...(options.headers || {}),
    },
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(body.message || "Server request failed");
  return body;
}

function mirrorSession(session) {
  const user = session.user
    ? {
        id: session.user.id,
        email: session.user.email,
        displayName: session.user.displayName || session.user.display_name || "",
        createdAt: session.user.createdAt || session.user.created_at || null,
      }
    : null;
  const mirrored = {
    accessToken: session.access_token || session.accessToken || null,
    expiresAt: session.expires_at || session.expiresAt || null,
    user,
  };
  writeJson(sessionKey, mirrored);
  return mirrored;
}

export function getSession() {
  return readJson(sessionKey, null);
}

export async function restoreSession() {
  if (!isDesktopRuntime()) return getSession();
  try {
    const session = await invoke("cloud_session");
    if (session) return mirrorSession(session);
    localStorage.removeItem(sessionKey);
    return null;
  } catch {
    return getSession();
  }
}

export async function clearSession() {
  localStorage.removeItem(sessionKey);
  if (isDesktopRuntime()) {
    await invoke("cloud_logout").catch(() => {});
  }
}

export async function registerAccount({ email, password, displayName }) {
  if (isDesktopRuntime()) {
    return mirrorSession(await invoke("cloud_register", { email, password, displayName }));
  }
  if (browserPreview()) {
    const users = readJson(usersKey, []);
    if (users.some((user) => user.email.toLowerCase() === email.toLowerCase())) {
      throw new Error("This email is already registered");
    }
    const user = { id: crypto.randomUUID(), email, displayName, passwordHash: await passwordDigest(password) };
    users.push(user);
    writeJson(usersKey, users);
    const session = { accessToken: `preview:${user.id}`, expiresAt: new Date(Date.now() + 86400000).toISOString(), user };
    writeJson(sessionKey, session);
    return session;
  }
  const session = await request("/v1/auth/register", {
    method: "POST",
    body: JSON.stringify({ email, password, display_name: displayName }),
  });
  return mirrorSession(session);
}

export async function loginAccount({ email, password }) {
  if (isDesktopRuntime()) {
    return mirrorSession(await invoke("cloud_login", { email, password }));
  }
  if (browserPreview()) {
    const user = readJson(usersKey, []).find((item) => item.email.toLowerCase() === email.toLowerCase());
    if (!user || user.passwordHash !== await passwordDigest(password)) throw new Error("Invalid email or password");
    const session = { accessToken: `preview:${user.id}`, expiresAt: new Date(Date.now() + 86400000).toISOString(), user };
    writeJson(sessionKey, session);
    return session;
  }
  const session = await request("/v1/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password, display_name: "Elyra user" }),
  });
  return mirrorSession(session);
}

export async function listNotifications() {
  if (isDesktopRuntime()) {
    return invoke("cloud_list_notifications");
  }
  if (browserPreview()) {
    const session = getSession();
    return readJson(notificationsKey, []).filter((item) => item.recipientEmail === session?.user?.email);
  }
  return request("/v1/notifications");
}

export async function createNotification({ recipientEmail, title, body }) {
  if (isDesktopRuntime()) {
    return invoke("cloud_send_notification", { recipientEmail, title, body });
  }
  if (browserPreview()) {
    const session = getSession();
    const users = readJson(usersKey, []);
    if (!session) throw new Error("Sign in to send a notification");
    if (!users.some((user) => user.email.toLowerCase() === recipientEmail.toLowerCase())) {
      throw new Error("Recipient is not registered in this preview");
    }
    const notifications = readJson(notificationsKey, []);
    const notification = {
      id: crypto.randomUUID(),
      senderId: session.user.id,
      senderEmail: session.user.email,
      recipientEmail,
      title,
      body,
      is_read: false,
      created_at: new Date().toISOString(),
    };
    notifications.unshift(notification);
    writeJson(notificationsKey, notifications);
    return notification;
  }
  return request("/v1/notifications", {
    method: "POST",
    body: JSON.stringify({ recipient_email: recipientEmail, title, body }),
  });
}

export async function markNotificationRead(id) {
  if (isDesktopRuntime()) {
    return invoke("cloud_mark_notification_read", { id });
  }
  if (browserPreview()) {
    const notifications = readJson(notificationsKey, []);
    writeJson(notificationsKey, notifications.map((item) => item.id === id ? { ...item, is_read: true } : item));
    return;
  }
  await request(`/v1/notifications/${encodeURIComponent(id)}/read`, { method: "PATCH" });
}
