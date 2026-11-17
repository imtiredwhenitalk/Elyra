import { createCryptoCard } from "../components/CryptoCard.js";
import { invoke } from "../services/tauri.js";
import {
  connectIntegration,
  deleteCryptoContact,
  exportPublicKey,
  generateCryptoKey,
  getShieldSummary,
  importPublicKey,
  listCryptoContacts,
  listCryptoKeys,
  listIntegrations,
  listSecurityAlerts,
  refreshDomainReputation,
  removeIntegrationKey,
  rotateCryptoKey,
  revokeCryptoKey,
  setIntegrationEnabled,
  testIntegration,
} from "../services/security.js";

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "'": "&#39;",
    '"': "&quot;",
  }[character]));
}

function icon(name) {
  return `<svg class="icon" aria-hidden="true"><use href="#i-${name}"></use></svg>`;
}

function notify(message, tone = "default") {
  const stack = document.querySelector("#toast-stack");
  if (!stack) return;
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

function formatStamp(value) {
  if (!value) return "never";
  const parsed = new Date(`${String(value).replace(" ", "T")}Z`);
  if (Number.isNaN(parsed.getTime())) return String(value);
  return parsed.toLocaleString([], { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

async function copyText(value) {
  try {
    await navigator.clipboard.writeText(value);
    return true;
  } catch {
    const area = document.createElement("textarea");
    area.value = value;
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.append(area);
    area.select();
    const copied = document.execCommand("copy");
    area.remove();
    return copied;
  }
}

function renderSecurityStatus(section, status, events) {
  const statusNode = section.querySelector("[data-security-status]");
  const checksNode = section.querySelector("[data-security-checks]");
  const auditNode = section.querySelector("[data-audit-events]");
  statusNode.textContent = status.status === "protected" ? "Protected" : "Needs attention";
  statusNode.dataset.status = status.status;
  checksNode.textContent = `${status.checks_passed}/${status.checks_total} controls passed · ${status.journal_mode.toUpperCase()} journal · ${status.audit_events} audit events`;
  auditNode.innerHTML = events.length
    ? events.map((event) => `<li><span class="audit-outcome ${event.outcome}">${event.outcome}</span><b>${event.event_type}</b><small>${escapeHtml(event.details)}</small></li>`).join("")
    : "<li>No security events recorded yet.</li>";
}

function integrationStatus(item) {
  if (!item.configured) return ["Not configured", "muted"];
  if (item.status === "connected") return ["Connected", "success"];
  if (item.status === "error") return ["Key rejected", "error"];
  if (item.status === "disabled") return ["Disabled", "muted"];
  return ["Unverified", "warning"];
}

function renderIntegrations(section, items) {
  const list = section.querySelector("[data-integrations]");
  list.innerHTML = items.map((item) => {
    const [label, tone] = integrationStatus(item);
    const details = item.lastError
      ? `<span class="integration-error">${escapeHtml(item.lastError)}</span>`
      : item.lastCheckedAt
        ? `<span class="integration-note">Last checked ${escapeHtml(formatStamp(item.lastCheckedAt))}</span>`
        : `<span class="integration-note">${escapeHtml(item.keyHint)}</span>`;
    return `
      <article class="integration-card" data-provider="${escapeHtml(item.provider)}">
        <div class="integration-head">
          <div>
            <h4>${escapeHtml(item.label)} <span class="integration-status ${tone}">${label}</span></h4>
            <p>${escapeHtml(item.description)}</p>
          </div>
          <a class="integration-docs" href="${escapeHtml(item.docsUrl)}" data-open-external="${escapeHtml(item.docsUrl)}">Get a key ${icon("arrow-right")}</a>
        </div>
        <div class="integration-key-row">
          <input type="password" data-integration-key placeholder="${escapeHtml(item.keyHint)}" autocomplete="off" spellcheck="false" aria-label="${escapeHtml(item.label)} API key" />
          <button class="ghost-button" type="button" data-integration-action="connect">${item.configured ? "Replace key" : "Save key"}</button>
        </div>
        <div class="integration-actions">
          ${item.configured ? `<button class="text-button" type="button" data-integration-action="test">${icon("refresh")} Test</button>` : ""}
          ${item.configured ? `<button class="text-button" type="button" data-integration-action="toggle">${item.enabled ? "Disable" : "Enable"}</button>` : ""}
          ${item.configured ? `<button class="text-button danger-text" type="button" data-integration-action="remove">${icon("trash")} Remove key</button>` : ""}
          ${details}
        </div>
      </article>
    `;
  }).join("");
}

function renderSummary(section, summary) {
  const grid = section.querySelector("[data-shield-summary]");
  grid.innerHTML = `
    <div><b>${summary.analyzed}</b><small>Messages analyzed</small></div>
    <div class="danger"><b>${summary.malicious}</b><small>Malicious</small></div>
    <div class="warning"><b>${summary.suspicious}</b><small>Suspicious</small></div>
    <div><b>${summary.providersConnected}</b><small>Providers connected</small></div>
    <div><b>${summary.falsePositives}</b><small>False positives reported</small></div>
  `;
  const toggle = section.querySelector("[data-external-scan]");
  if (toggle) toggle.checked = summary.externalScanning !== false;
}

function renderAlerts(section, alerts) {
  const list = section.querySelector("[data-shield-alerts]");
  if (!alerts.length) {
    list.innerHTML = "<li>No suspicious messages. Elyra Shield has not flagged anything.</li>";
    return;
  }
  list.innerHTML = alerts.map((alert) => `
    <li class="shield-alert ${escapeHtml(alert.risk)}">
      <span class="threat-chip ${escapeHtml(alert.risk)}">${escapeHtml(alert.risk)}</span>
      <div>
        <b>${escapeHtml(alert.subject)}</b>
        <small>${escapeHtml(alert.sender)} · score ${alert.score}/100${alert.falsePositive ? " · reported as false positive" : ""}</small>
      </div>
    </li>
  `).join("");
}

function keyStatusLabel(status) {
  if (status === "active") return ["Active", "success"];
  if (status === "retired") return ["Retired", "muted"];
  return ["Revoked", "error"];
}

function renderKeys(section, keys, contacts) {
  const list = section.querySelector("[data-crypto-keys]");
  if (!keys.length) {
    list.innerHTML = `<div class="crypto-empty"><b>No identity key yet</b><span>Generate an Ed25519 key pair. The private key is written to the Windows Credential Manager and never leaves this device.</span></div>`;
  } else {
    list.innerHTML = keys.map((key) => {
      const [label, tone] = keyStatusLabel(key.status);
      return `
        <article class="crypto-key ${escapeHtml(key.status)}">
          <div class="crypto-key-head">
            <div><b>${escapeHtml(key.label)}</b><span class="integration-status ${tone}">${label}</span></div>
            <code title="Public key fingerprint">${escapeHtml(key.fingerprint)}</code>
          </div>
          <small>${escapeHtml(key.algorithm)} · created ${escapeHtml(formatStamp(key.createdAt))} · private key ${key.hasPrivateKey ? "stored in the OS keychain" : "not available"}</small>
          <div class="crypto-key-actions">
            <button class="text-button" type="button" data-key-action="export" data-key-id="${key.id}">${icon("download")} Copy public key</button>
            ${key.status === "active" ? `<button class="text-button" type="button" data-key-action="rotate" data-key-id="${key.id}">${icon("refresh")} Rotate</button><button class="text-button danger-text" type="button" data-key-action="revoke" data-key-id="${key.id}">${icon("x")} Revoke</button>` : ""}
          </div>
        </article>
      `;
    }).join("");
  }

  const generate = section.querySelector("[data-key-generate]");
  if (generate) {
    const active = keys.find((key) => key.status === "active");
    generate.disabled = Boolean(active);
    generate.title = active
      ? "Rotate the active identity key instead of generating a second one."
      : "Generate an Ed25519 identity key pair.";
  }

  const contactsNode = section.querySelector("[data-crypto-contacts]");
  contactsNode.innerHTML = contacts.length
    ? contacts.map((contact) => `
        <li>
          <div><b>${escapeHtml(contact.label)}</b><small>${escapeHtml(contact.email)} · ${escapeHtml(contact.fingerprint)}</small></div>
          <button class="icon-button tiny" type="button" data-contact-delete="${contact.id}" title="Remove contact key">${icon("x")}</button>
        </li>
      `).join("")
    : "<li>No contact public keys imported yet.</li>";
}

async function refreshIntegrations(section) {
  const [items, summary] = await Promise.all([listIntegrations(), getShieldSummary()]);
  renderIntegrations(section, items);
  renderSummary(section, summary);
}

async function refreshKeys(section) {
  const [keys, contacts] = await Promise.all([listCryptoKeys(), listCryptoContacts()]);
  renderKeys(section, keys, contacts);
}

async function loadSecurityStatus(section) {
  try {
    const [status, events, summary, alerts] = await Promise.all([
      invoke("get_security_status"),
      invoke("list_audit_events", { limit: 12 }),
      getShieldSummary(),
      listSecurityAlerts(5),
    ]);
    renderSecurityStatus(section, status, events);
    renderSummary(section, summary);
    renderAlerts(section, alerts);
  } catch (error) {
    const statusNode = section.querySelector("[data-security-status]");
    statusNode.textContent = "Unavailable";
    statusNode.dataset.status = "attention";
    console.error("Security status check failed", error);
  }
}

function bindIntegrationActions(section) {
  section.addEventListener("click", async (event) => {
    const reputationButton = event.target.closest("[data-refresh-reputations]");
    if (reputationButton) {
      reputationButton.disabled = true;
      try {
        const refreshed = await refreshDomainReputation("");
        notify(`${refreshed} sender domain${refreshed === 1 ? "" : "s"} refreshed`, "success");
        await loadSecurityStatus(section);
      } catch (error) {
        notify(error.message || "Could not refresh sender reputations", "error");
      } finally {
        reputationButton.disabled = false;
      }
      return;
    }

    const external = event.target.closest("[data-open-external]");
    if (external) {
      event.preventDefault();
      const url = external.dataset.openExternal;
      const opener = window.__TAURI__?.opener;
      if (opener?.openUrl) await opener.openUrl(url).catch(() => {});
      else window.open(url, "_blank", "noopener,noreferrer");
      return;
    }

    const button = event.target.closest("[data-integration-action]");
    if (button) {
      const card = button.closest("[data-provider]");
      const provider = card?.dataset.provider;
      if (!provider) return;
      const action = button.dataset.integrationAction;
      const keyInput = card.querySelector("[data-integration-key]");
      button.disabled = true;
      try {
        let view;
        if (action === "connect") {
          const key = keyInput?.value.trim() || "";
          if (!key) {
            notify("Paste the API key first", "error");
            return;
          }
          view = await connectIntegration(provider, key);
          if (view.status === "connected") {
            if (keyInput) keyInput.value = "";
            notify(`${view.label} connected`, "success");
          } else {
            notify(view.lastError || `${view.label} rejected the API key`, "error");
          }
        } else if (action === "test") {
          view = await testIntegration(provider);
          if (view.status === "connected") notify(`${view.label} accepted the key`, "success");
          else notify(view.lastError || `${view.label} test failed`, "error");
        } else if (action === "toggle") {
          const items = await listIntegrations();
          const current = items.find((item) => item.provider === provider);
          view = await setIntegrationEnabled(provider, !current?.enabled);
          notify(`${view.label} ${view.enabled ? "enabled" : "disabled"}`, "success");
        } else if (action === "remove") {
          if (!window.confirm("Remove this API key from the OS keychain?")) return;
          view = await removeIntegrationKey(provider);
          notify(`${view.label} key removed`, "success");
        }
        await refreshIntegrations(section);
      } catch (error) {
        notify(error.message || "The integration request failed", "error");
        await refreshIntegrations(section).catch(() => {});
      } finally {
        button.disabled = false;
      }
      return;
    }

    const keyButton = event.target.closest("[data-key-action]");
    if (keyButton) {
      const id = Number(keyButton.dataset.keyId);
      const action = keyButton.dataset.keyAction;
      keyButton.disabled = true;
      try {
        if (action === "export") {
          const armored = await exportPublicKey(id);
          const copied = await copyText(armored);
          notify(copied ? "Public key copied to the clipboard" : "Could not copy the public key", copied ? "success" : "error");
        } else if (action === "rotate") {
          if (!window.confirm("Rotate this identity key? The current key becomes retired and its private material is deleted.")) return;
          await rotateCryptoKey(id);
          notify("Identity key rotated", "success");
          await refreshKeys(section);
        } else if (action === "revoke") {
          if (!window.confirm("Revoke this identity key? This cannot be undone.")) return;
          await revokeCryptoKey(id);
          notify("Identity key revoked", "success");
          await refreshKeys(section);
        }
      } catch (error) {
        notify(error.message || "The key operation failed", "error");
      } finally {
        keyButton.disabled = false;
      }
      return;
    }

    const generate = event.target.closest("[data-key-generate]");
    if (generate) {
      generate.disabled = true;
      try {
        await generateCryptoKey("Primary identity");
        notify("Identity key generated in the OS keychain", "success");
        await refreshKeys(section);
      } catch (error) {
        notify(error.message || "Could not generate the key", "error");
      } finally {
        generate.disabled = false;
      }
      return;
    }

    const importButton = event.target.closest("[data-key-import]");
    if (importButton) {
      const email = section.querySelector("[data-contact-email]").value.trim();
      const label = section.querySelector("[data-contact-label]").value.trim();
      const publicKey = section.querySelector("[data-contact-key]").value.trim();
      if (!email || !publicKey) {
        notify("Enter the contact email and public key", "error");
        return;
      }
      importButton.disabled = true;
      try {
        await importPublicKey(email, label, publicKey);
        section.querySelector("[data-contact-email]").value = "";
        section.querySelector("[data-contact-label]").value = "";
        section.querySelector("[data-contact-key]").value = "";
        notify("Contact public key imported", "success");
        await refreshKeys(section);
      } catch (error) {
        notify(error.message || "Could not import the public key", "error");
      } finally {
        importButton.disabled = false;
      }
      return;
    }

    const removeContact = event.target.closest("[data-contact-delete]");
    if (removeContact) {
      if (!window.confirm("Remove this contact public key?")) return;
      try {
        await deleteCryptoContact(Number(removeContact.dataset.contactDelete));
        notify("Contact key removed", "success");
        await refreshKeys(section);
      } catch (error) {
        notify(error.message || "Could not remove the contact key", "error");
      }
    }
  });
}

export function mountSecurityPage(container) {
  if (!container || container.querySelector("[data-security-page]")) return;
  const section = document.createElement("div");
  section.className = "security-page-section";
  section.dataset.securityPage = "true";
  section.innerHTML = `
    <div class="security-page-heading">
      <div>
        <span class="eyebrow">Elyra Shield</span>
        <h3>Protect your local workspace</h3>
        <p>Content analysis runs in the Rust core. External link checks use your own provider keys.</p>
      </div>
      <label class="security-toggle"><input type="checkbox" data-external-scan checked /><span>External link scanning</span></label>
    </div>

    <div class="security-status-panel">
      <div><small>Device security posture</small><strong data-security-status>Checking...</strong><span data-security-checks>Running local policy checks</span></div>
      <button class="security-action" data-security-refresh type="button">Run checks <span>→</span></button>
    </div>

    <div class="shield-summary" data-shield-summary></div>

    <section class="security-card">
      <div class="security-card-heading">
        <span class="security-icon">${icon("shield")}</span>
        <div><h3>Threat intelligence</h3><p>Connect your own VirusTotal and Google Safe Browsing keys. Keys are stored in the Windows Credential Manager and never returned to the interface.</p></div>
      </div>
      <div class="integration-list" data-integrations></div>
    </section>

    <section class="security-card">
      <div class="security-card-heading">
        <span class="security-icon">${icon("alert")}</span>
        <div><h3>Recent Shield alerts</h3><p>Unified risk results for messages that were analyzed locally or with external providers.</p></div>
      </div>
      <ul class="shield-alert-list" data-shield-alerts><li>Loading Shield results...</li></ul>
      <div class="panel-actions-row">
        <button class="text-button" type="button" data-refresh-reputations>${icon("refresh")} Refresh sender reputations</button>
      </div>
    </section>

    <section class="security-card">
      <div class="security-card-heading">
        <span class="security-icon">${icon("key")}</span>
        <div><h3>Identity keys</h3><p>Ed25519 key pairs for signing and future end-to-end encryption. Private keys stay in OS secure storage.</p></div>
      </div>
      <div class="crypto-key-list" data-crypto-keys></div>
      <div class="panel-actions-row wrap">
        <button class="ghost-button" type="button" data-key-generate>${icon("plus")} Generate identity key</button>
      </div>
      <div class="crypto-contact-form">
        <div class="field-grid">
          <label>Contact email<input type="email" data-contact-email placeholder="person@example.com" autocomplete="off" /></label>
          <label>Label<input type="text" data-contact-label placeholder="Work contact" autocomplete="off" /></label>
        </div>
        <label>Public key<textarea data-contact-key rows="3" placeholder="Paste an Elyra public key" spellcheck="false"></textarea></label>
        <button class="ghost-button" type="button" data-key-import>${icon("user-plus")} Import public key</button>
      </div>
      <ul class="crypto-contact-list" data-crypto-contacts><li>No contact public keys imported yet.</li></ul>
    </section>

    <div class="audit-panel">
      <div class="security-card-heading"><div><h3>Security activity</h3><p>Local events only. Secrets and message bodies are never recorded.</p></div></div>
      <ul class="audit-events" data-audit-events><li>Loading audit events...</li></ul>
    </div>
  `;
  container.append(section);
  section.append(createCryptoCard());

  section.querySelector("[data-external-scan]").addEventListener("change", async (event) => {
    const enabled = event.target.checked;
    try {
      await invoke("set_local_setting", { key: "external_scanning", value: enabled ? "true" : "false" });
      notify(enabled ? "External link scanning enabled" : "External link scanning disabled", "success");
      await refreshIntegrations(section);
    } catch (error) {
      event.target.checked = !enabled;
      notify(error.message || "Could not save the preference", "error");
    }
  });
  section.querySelector("[data-security-refresh]").addEventListener("click", () => loadSecurityStatus(section));
  bindIntegrationActions(section);

  invoke("get_local_settings")
    .then((settings) => {
      const toggle = section.querySelector("[data-external-scan]");
      if (toggle) toggle.checked = String(settings.external_scanning) !== "false";
    })
    .catch(() => {});

  loadSecurityStatus(section);
  refreshIntegrations(section).catch(() => {});
  refreshKeys(section).catch((error) => console.error("Could not load identity keys", error));
}
