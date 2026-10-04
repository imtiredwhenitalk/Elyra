# `elyra-crypto`

Reusable Rust cryptographic primitives for Elyra. It provides AES-256-GCM
authenticated encryption, OS-random key generation, payload validation, and
SHA-256 fingerprints.

Keys are zeroized when dropped. This package is for local secrets only; the
server must never receive encryption keys.
