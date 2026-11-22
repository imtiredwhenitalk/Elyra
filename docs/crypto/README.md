# Elyra crypto layer

The desktop crypto boundary lives in `apps/desktop/src-tauri/src/crypto.rs`. Browser code can request operations through the Tauri commands exposed by `lib.rs`, but it never implements encryption itself.

## Current guarantees

- AES-256-GCM authenticated encryption.
- 32-byte keys generated with `OsRng`.
- A fresh 96-bit nonce for every encryption operation.
- The nonce is stored as part of the base64 payload; it is not secret.
- SHA-256 fingerprints are for display and comparison only, not authentication.
- Invalid keys, malformed payloads, wrong keys, and tampered ciphertext return errors.
- RFC 9180 HPKE with DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, and AES-256-GCM.
- Typed, versioned message envelopes with Ed25519 signatures over canonical envelope metadata and ciphertext.
- Recipient payload keys are sealed separately for every recipient X25519 key.

The current Security Lab deliberately keeps generated keys in memory and uses a known test message. Production account keys still need OS keychain integration before encrypting mailbox data at rest. Never put encryption keys in localStorage, URLs, logs, or email content.

The planned E2EE protocol, trust boundaries, key separation, recipient
verification, and forward-secrecy limitations are defined in
[`../security/e2ee.md`](../security/e2ee.md).