import { decryptText, encryptText, fingerprint, generateEncryptionKey } from "../services/crypto.js";

export function createCryptoCard() {
  const card = document.createElement("section");
  card.className = "security-card";
  card.innerHTML = `<div class="security-card-heading"><span class="security-icon">⌾</span><div><h3>Local encryption</h3><p>Test the Rust AES-256-GCM engine without storing your key.</p></div><span class="security-state">Ready</span></div><div class="security-card-body"><div><small>Key fingerprint</small><strong data-fingerprint>Not generated</strong></div><div><small>Round-trip check</small><strong data-result>Not run</strong></div></div><button class="security-action" type="button">Generate key and run check <span>→</span></button><p class="security-note">The generated key exists only in this page session.</p>`;
  const action = card.querySelector(".security-action");
  const fingerprintNode = card.querySelector("[data-fingerprint]");
  const resultNode = card.querySelector("[data-result]");
  action.addEventListener("click", async () => {
    action.disabled = true;
    action.textContent = "Running crypto check...";
    resultNode.textContent = "Working";
    try {
      const key = await generateEncryptionKey();
      const encrypted = await encryptText(key, "Elyra private note");
      const decrypted = await decryptText(key, encrypted);
      fingerprintNode.textContent = await fingerprint(key);
      resultNode.textContent = decrypted === "Elyra private note" ? "Passed" : "Mismatch";
      resultNode.dataset.status = decrypted === "Elyra private note" ? "success" : "error";
    } catch (error) {
      resultNode.textContent = "Unavailable";
      resultNode.dataset.status = "error";
      console.error("Crypto check failed", error);
    } finally {
      action.disabled = false;
      action.innerHTML = "Run encryption check again <span>→</span>";
    }
  });
  return card;
}
