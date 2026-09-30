import { createCryptoCard } from "../components/CryptoCard.js";

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
  section.innerHTML = `<div class="security-page-heading"><div><span class="eyebrow">Security lab</span><h3>Protect your local workspace</h3><p>Encryption is executed by the Rust core, not by browser JavaScript.</p></div><label class="security-toggle"><input type="checkbox" ${localOnly ? "checked" : ""} /><span>Local-only mode</span></label></div>`;
  const toggle = section.querySelector("input");
  toggle.addEventListener("change", () => localStorage.setItem("elyra-security-preferences", JSON.stringify({ localOnly: toggle.checked })));
  section.append(createCryptoCard());
  container.append(section);
}
