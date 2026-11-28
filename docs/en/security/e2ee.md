> 🌐 **English** · [Українська](../../ua/security/e2ee.md)

# Elyra End-to-End Encryption Architecture

## Status

This document defines the initial protocol direction for Elyra Issue #6.
It is an architecture specification, not a claim that E2EE is implemented or
production-ready.

The current `elyra-crypto` AES-256-GCM primitive encrypts local test data only.
It does not perform identity verification, key agreement, message authentication
between users, or secure mail delivery.

## Goals

1. Keep message plaintext and private keys outside Elyra server storage.
2. Encrypt a message once for every authorized Elyra recipient.
3. Authenticate sender identity and detect tampering.
4. Use established cryptographic protocols and vetted libraries.
5. Separate signing identities from encryption keys.
6. Define encrypted message versions and fail closed on unsupported versions.
7. Document metadata and recovery limitations honestly.

## Assets and trust boundaries

### Desktop trusted client

The desktop Rust boundary owns private keys, key agreement, envelope
construction, plaintext rendering, and decryption.

The Tauri webview may request cryptographic operations but must not receive
private signing or encryption keys. It may receive public keys, fingerprints,
verification state, ciphertext, and decryption output.

### OS keychain

Private Ed25519 signing keys and private encryption keys belong in the platform
keychain. Existing keychain entries use `crypto-key-<id>`. Each key record in
SQLite contains its public key, fingerprint, status, and lifecycle timestamps.

Private key material must never be written to SQLite, localStorage, URLs, logs,
audit events, or API payloads.

### Elyra server

The server is an untrusted transport and metadata service. Its authorized scope
is:

- encrypted message envelopes;
- recipient identifiers;
- key identifiers and versions;
- routing metadata;
- delivery and retention timestamps.

The server must not receive:

- plaintext message bodies or subjects;
- private keys or recovery secrets;
- plaintext attachment contents;
- session secret material.

## Protocol decision

The initial Elyra protocol uses RFC 9180 HPKE with this ciphersuite:

- KEM: DHKEM(X25519, HKDF-SHA256);
- KDF: HKDF-SHA256;
- AEAD: AES-256-GCM;
- HPKE mode: Base;
- sender authentication: separate Ed25519 signature over the canonical envelope;
- key separation: Ed25519 signing keys and X25519 encryption keys are distinct
  key records with independent lifecycles.

The selected implementation is the maintained Rust `hpke` crate, version
`0.15.1`. The initial dependency configuration will disable default features
and enable only `alloc`, `getrandom`, `x25519`, and `aes`. This excludes unused
ChaCha20-Poly1305, ML-KEM, X-Wing, and SHAKE implementations from Elyra builds.
The exact suite types are:

```rust
type Kem = hpke::kem::X25519HkdfSha256;
type Kdf = hpke::kdf::HkdfSha256;
type Aead = hpke::aead::AesGcm256;
```

Ed25519 is a signing algorithm, not an encryption key. HPKE Base mode authenticates
to the selected recipient key but does not authenticate the sender. Elyra will
therefore sign the complete canonical envelope with an Ed25519 identity key
and verify that key using a verified recipient or sender trust record.

No custom cryptographic primitive composition or nonce derivation scheme is
allowed. HPKE defines the encapsulation and KDF behavior, while the AES-GCM
payload is additionally bound to canonical envelope AAD.

### Sender workflow

1. Authenticate and obtain verified recipient public encryption keys.
2. Generate a random per-message symmetric payload key.
3. Encrypt the payload key to each recipient with HPKE.
4. Encrypt the body and attachment metadata with AES-256-GCM.
5. Include a cleartext envelope identifying format version and key versions.
6. Canonicalize and sign the format version, sender identity key ID, recipient
   key IDs, HPKE recipient ciphertexts, payload ID, nonce, and authentication
   metadata with Ed25519.
7. Transmit only ciphertext and metadata to the server.

The payload nonce must be unique per encryption. A cryptographic nonce must
never be derived from predictable or reused state.

The AES-GCM payload must use the canonical envelope fields as additional
authenticated data (AAD), or an equivalent binding that prevents replacing one
signed envelope with another. The exact encoding must be deterministic and
specified before implementation.

### Recipient workflow

1. Fetch the encrypted envelope.
2. Reject unsupported format or algorithm versions.
3. Verify recipient, key, and sender identity constraints.
4. Verify the Ed25519 signature using a verified sender key.
5. Open the HPKE recipient envelope with the matching private key.
6. Authenticate and decrypt the payload.
7. Reject replayed or corrupt payloads without rendering plaintext.

## Versioned message envelope

The implementation must use a typed, versioned envelope. The first format should
contain:

- `version`;
- `sender_identity_key_id`;
- `sender_signature`;
- `recipient_key_ids`;
- HPKE recipient ciphertexts;
- encrypted payload identifier;
- encrypted message nonce;
- authentication metadata;
- creation and expiration information, if retention policy requires it.

Plaintext subjects, file names, attachment names, and message bodies must not
appear in the server-facing envelope.

Downgrade is prohibited: ciphertext marked for a newer Elyra format must not be
accepted as a lower-format plaintext or legacy SMTP message.

## Key lifecycle

### Signing keys

Ed25519 signing keys remain the identity key. Rotation creates a new active key,
retains retirement metadata, and removes only the old private key after the
policy-defined grace period.

Revocation prevents trust for new messages. It must not silently transform old
valid signatures into invalid historical records.

### Encryption keys

Encryption keys are X25519 key pairs managed independently from signing keys.

Each key record must track:

- key identifier;
- algorithm;
- public key;
- private key location;
- fingerprint;
- status: active, retired, revoked;
- creation, retirement, and revocation timestamps.

Revoked recipient keys must not receive new ciphertext. A recent key change must
produce a visible recipient warning.

## Recipient identity verification

The server must not be the sole source of recipient identity trust.

Users must be able to compare local fingerprints manually and mark keys as:

- unverified;
- verified;
- changed;
- revoked.

Unexpected recipient public-key changes must block or warn before sending.
The UI must not silently replace a verified key.

## Forward secrecy

HPKE recipient envelopes provide authenticated per-message key encapsulation,
but HPKE alone does not provide asynchronous forward secrecy or
post-compromise security for a continuing chat session.

Forward secrecy remains a separate milestone. It requires session or ratchet
design, out-of-order delivery handling, recovery behavior, and independent
security review. It must not be claimed before that implementation is complete.

## Attachments

Each attachment should receive independent encryption material and an integrity
tag. The payload should contain or authenticate:

- original filename;
- MIME type;
- file size;
- ciphertext;
- attachment-specific key identifiers or content encryption key.

The server may enforce size limits but must not inspect plaintext attachment
content. Existing threat-intelligence and attachment-scanning behavior must
preserve explicit privacy and consent boundaries.

## Metadata

The server and network may learn:

- account identifiers;
- sender and recipient pseudonymous/routing identifiers;
- message size and timing;
- delivery timestamps;
- key identifiers and key-change events;
- IP addresses and SMTP provider metadata;
- encrypted attachment size.

The E2EE implementation must not claim protection of metadata that remains
available to the server or network.

## Replay and downgrade controls

The implementation must reject:

- unsupported envelope versions;
- duplicate message identifiers;
- ciphertext for a revoked or wrong recipient;
- invalid Ed25519 signatures;
- altered recipient key identifiers;
- payload authentication failures;
- plaintext fallback without explicit user consent.

Replay protection uses server-side or client-side message identifiers plus
authenticated envelope fields. Exact storage and retention policy belongs in
the server-envelope design.

## Server delivery model

Elyra accounts may initially store ciphertext envelopes in PostgreSQL and route
notifications containing no plaintext.

The server relay must never accept an Elyra E2EE plaintext body for persistence
or server-side processing. External SMTP/Elyra fallback behavior requires a
separate, explicit compatibility decision.

## OpenPGP interoperability

OpenPGP, RFC 9580, should be evaluated for interoperability with external
mail clients and key imports.

OpenPGP compatibility is a later milestone. It must not weaken Elyra's internal
key identifiers, verification states, or downgrade protections. Any external
OpenPGP implementation must use a maintained library and independently reviewed
payload formats.

Sending ordinary external SMTP plaintext must require explicit user awareness.
Elyra must never silently fall back from Elyra E2EE to plaintext SMTP.

## Threat model

### Protected by the architecture

- Server database cannot read message plaintext.
- Invalid, truncated, or tampered payloads fail authentication.
- Private keys remain on the trusted device or platform keychain.
- Recipient key substitution requires a verification-state change.
- Unsupported protocol versions fail closed.
- Audit logs and API responses do not contain plaintext or private keys.

### Not protected

- A compromised desktop OS or malware with access to the unlocked keychain.
- An actively malicious verified recipient or sender device.
- Server and network metadata described above.
- Mail provider metadata for relayed or external SMTP mail.
- Lost private keys and messages without a defined recovery mechanism.
- Forward secrecy until a ratchet or equivalent session protocol is implemented.
- Recovery-service compromise until encrypted backup and recovery design exists.

## Failure behavior

- Invalid key: reject before envelope creation.
- Missing recipient key: fail or warn; never use a silent fallback.
- Changed key: require warning or verification.
- Revoked key: reject new encryption.
- Unsupported envelope: reject.
- Authentication failure: do not render plaintext.
- Missing private key: report actionable key recovery or unavailable state.
- Storage failure: surface an error; do not claim delivery or encryption success.

## Implementation milestones

1. Add X25519 recipient key-pair generation and key separation.
2. Select and lock vetted HPKE/AES-GCM dependencies.
3. Implement identity-signature verification and recipient trust states.
4. Implement versioned envelope encryption/decryption in Rust.
5. Update server storage and routing for ciphertext-only payloads.
6. Encrypt attachments with independent content keys.
7. Add replay, downgrade, key-rotation, revocation, and malformed-envelope tests.
8. Add key recovery and multi-device authorization only with a reviewed design.
9. Consider forward secrecy separately from the initial asynchronous protocol.
10. Complete independent cryptographic review before production E2EE claims.