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
  return !(window.__TAURI__?.core?.invoke) && !window.__ELYRA_API_URL__;
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

export function getSession() {
  return readJson(sessionKey, null);
}

export function clearSession() {
  localStorage.removeItem(sessionKey);
}

export async function registerAccount({ email, password, displayName }) {
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
  writeJson(sessionKey, { accessToken: session.access_token, expiresAt: session.expires_at, user: session.user });
  return getSession();
}

export async function loginAccount({ email, password }) {
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
  writeJson(sessionKey, { accessToken: session.access_token, expiresAt: session.expires_at, user: session.user });
  return getSession();
}

export async function listNotifications() {
  if (browserPreview()) {
    const session = getSession();
    return readJson(notificationsKey, []).filter((item) => item.recipientEmail === session?.user?.email);
  }
  return request("/v1/notifications");
}

export async function createNotification({ recipientEmail, title, body }) {
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
  if (browserPreview()) {
    const notifications = readJson(notificationsKey, []);
    writeJson(notificationsKey, notifications.map((item) => item.id === id ? { ...item, is_read: true } : item));
    return;
  }
  await request(`/v1/notifications/${encodeURIComponent(id)}/read`, { method: "PATCH" });
}
