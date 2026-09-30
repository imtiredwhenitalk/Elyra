import { invoke } from "@tauri-apps/api/core";

export function generateEncryptionKey() {
  return invoke("generate_encryption_key");
}

export function encryptText(key, plaintext) {
  return invoke("encrypt_text", { key, plaintext });
}

export function decryptText(key, payload) {
  return invoke("decrypt_text", { key, payload });
}

export function fingerprint(value) {
  return invoke("fingerprint", { value });
}
