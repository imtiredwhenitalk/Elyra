import { createCryptoCard } from "../components/CryptoCard.js";
import { usePersistentState } from "../hooks/usePersistentState.js";

export function mountSecurityPage(container) {
  if (!container || container.querySelector(".security-card")) return;
  const preferences = usePersistentState("elyra-security-preferences", { localOnly: true });
  const section = document.createElement("div");
  section.className = "security-page-section";
  section.innerHTML = `<div class="security-page-heading"><div><span class="eyebrow">Security lab</span><h3>Protect your local workspace</h3><p>Encryption is executed by the Rust core, not by browser JavaScript.</p></div><label class="security-toggle"><input type="checkbox" ${preferences.value.localOnly ? "checked" : ""} /><span>Local-only mode</span></label></div>`;
  const toggle = section.querySelector("input");
  toggle.addEventListener("change", () => preferences.set({ localOnly: toggle.checked }));
  section.append(createCryptoCard());
  container.append(section);
}
