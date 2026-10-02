import { createCryptoCard } from "../components/CryptoCard.js";
import { invoke } from "../services/tauri.js";

function renderSecurityStatus(section, status, events) {
  const statusNode = section.querySelector("[data-security-status]");
  const checksNode = section.querySelector("[data-security-checks]");
  const auditNode = section.querySelector("[data-audit-events]");
  statusNode.textContent = status.status === "protected" ? "Protected" : "Needs attention";
  statusNode.dataset.status = status.status;
  checksNode.textContent = `${status.checks_passed}/${status.checks_total} controls passed · ${status.journal_mode.toUpperCase()} journal · ${status.audit_events} audit events`;
  auditNode.innerHTML = events.length
    ? events.map((event) => `<li><span class="audit-outcome ${event.outcome}">${event.outcome}</span><b>${event.event_type}</b><small>${event.details}</small></li>`).join("")
    : "<li>No security events recorded yet.</li>";
}

async function loadSecurityStatus(section) {
  try {
    const [status, events] = await Promise.all([invoke("get_security_status"), invoke("list_audit_events", { limit: 12 })]);
    renderSecurityStatus(section, status, events);
  } catch (error) {
    const statusNode = section.querySelector("[data-security-status]");
    statusNode.textContent = "Unavailable";
    statusNode.dataset.status = "attention";
    console.error("Security status check failed", error);
  }
}

export function mountSecurityPage(container) {
  if (!container || container.querySelector(".security-card")) return;
  const stored = localStorage.getItem("elyra-security-preferences");
  let localOnly = true;
  try {
    localOnly = stored ? JSON.parse(stored).localOnly !== false : true;
  } catch {
    localOnly = true;
  }
  const section = document.createElement("div");
  section.className = "security-page-section";
  section.innerHTML = `<div class="security-page-heading"><div><span class="eyebrow">Security lab</span><h3>Protect your local workspace</h3><p>Encryption is executed by the Rust core, not by browser JavaScript.</p></div><label class="security-toggle"><input type="checkbox" ${localOnly ? "checked" : ""} /><span>Local-only mode</span></label></div><div class="security-status-panel"><div><small>Device security posture</small><strong data-security-status>Checking...</strong><span data-security-checks>Running local policy checks</span></div><button class="security-action" data-security-refresh type="button">Run checks <span>→</span></button></div><div class="audit-panel"><div class="security-card-heading"><div><h3>Security activity</h3><p>Local events only. Secrets and message bodies are never recorded.</p></div></div><ul class="audit-events" data-audit-events><li>Loading audit events...</li></ul></div>`;
  const toggle = section.querySelector("input");
  toggle.addEventListener("change", () => localStorage.setItem("elyra-security-preferences", JSON.stringify({ localOnly: toggle.checked })));
  section.querySelector("[data-security-refresh]").addEventListener("click", () => loadSecurityStatus(section));
  section.append(createCryptoCard());
  container.append(section);
  loadSecurityStatus(section);
}
