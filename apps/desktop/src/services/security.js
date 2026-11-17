import { invoke } from "./tauri.js";

export function listIntegrations() {
  return invoke("list_integrations");
}

export function connectIntegration(provider, key) {
  return invoke("connect_integration", { provider, key });
}

export function testIntegration(provider) {
  return invoke("test_integration", { provider });
}

export function setIntegrationEnabled(provider, enabled) {
  return invoke("set_integration_enabled", { provider, enabled });
}

export function removeIntegrationKey(provider) {
  return invoke("remove_integration_key", { provider });
}

export function scanLinks(text) {
  return invoke("scan_links", { text });
}

export function analyzeMessage(id, rescan = false) {
  return invoke("analyze_message", { id, rescan });
}

export function getMessageSecurity(id) {
  return invoke("get_message_security", { id });
}

export function listSecurityAlerts(limit = 25) {
  return invoke("list_security_alerts", { limit });
}

export function getShieldSummary() {
  return invoke("get_shield_summary");
}

export function reportFalsePositive(id, note = "") {
  return invoke("report_false_positive", { id, note });
}

export function listCryptoKeys() {
  return invoke("list_crypto_keys");
}

export function generateCryptoKey(label = "") {
  return invoke("generate_crypto_key", { label });
}

export function rotateCryptoKey(id, label = "") {
  return invoke("rotate_crypto_key", { id, label });
}

export function revokeCryptoKey(id) {
  return invoke("revoke_crypto_key", { id });
}

export function exportPublicKey(id) {
  return invoke("export_public_key", { id });
}

export function importPublicKey(email, label, publicKey) {
  return invoke("import_public_key", { email, label, publicKey });
}

export function listCryptoContacts() {
  return invoke("list_crypto_contacts");
}

export function deleteCryptoContact(id) {
  return invoke("delete_crypto_contact", { id });
}
