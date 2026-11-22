import { invoke } from "./tauri.js";

export function generateEncryptionKey() {
  return invoke("generate_encryption_key");
}

export function encryptText(key, plaintext) {
  return invoke("encrypt_text", { key, plaintext });
}

export function decryptText(key, payload) {
  return invoke("decrypt_text", { key, payload });
}

export function encryptTextForRecipient(recipientId, key, plaintext) {
  return invoke("encrypt_text_for_recipient", { recipientId, key, plaintext });
}

export function sealMessageEnvelope(recipientId, plaintext) {
  return invoke("seal_message_envelope", { recipientId, plaintext });
}

export function openMessageEnvelope(envelopeJson) {
  return invoke("open_message_envelope", { envelopeJson });
}

export function importEncryptionPublicKey(contactId, publicKey) {
  return invoke("import_crypto_encryption_public_key", { contactId, publicKey });
}

export function fingerprint(value) {
  return invoke("fingerprint", { value });
}
