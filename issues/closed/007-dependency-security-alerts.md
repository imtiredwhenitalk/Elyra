# Dependency security alerts

- Status: done
- Labels: security, dependencies
- Area: `Cargo.lock`, `server/Cargo.toml`, `apps/desktop/src-tauri/Cargo.toml`, `providers.rs`, `auth.rs`

## Problem

GitHub reported 13 open Dependabot alerts (6 moderate, 7 low) on `main`:

- `jsonwebtoken` 9.3.1 — type confusion leading to potential authorization bypass.
- `lexical-core` 0.7.6 — multiple soundness issues, reached through `imap` 2.4's `nom` 5 parser.
- `glib` 0.18.5 — unsound `VariantStrIter` iterator implementations.
- `rustls` 0.23.45 — ECH data exposure, hybrid key-exchange poisoning, secret leakage through `Debug`, and attacker-controlled Brotli decompression.

Two alerts came from a stale nested `apps/desktop/src-tauri/Cargo.lock`; the
workspace root lockfile is the one used by CI.

## Fix

- Upgraded `jsonwebtoken` to 10.4.0 and pinned validation to `HS256` explicitly.
- Replaced `imap` 2.4 with 3.0.0-alpha.15, which uses `nom` 7 and removes
  `lexical-core` from the graph. `providers.rs` now uses `ClientBuilder` and
  `imap::Connection`; the old `native-tls` connector is gone.
- Upgraded Tauri to 2.13.0, `tauri-build` to 2.8.0, and
  `tauri-plugin-opener` to 2.7.3. Tauri 2.13 uses GTK 0.19, which pulls
  `glib` 0.22.11 instead of the vulnerable 0.18 series.
- Updated `rustls` to 0.23.46 in the workspace lockfile.
- Deleted the stale nested desktop lockfile.
- Added `.github/dependabot.yml` for weekly Cargo and npm update PRs.

## Validation

- `cargo metadata --locked --no-deps` passes.
- Lockfile audit: `rustls 0.23.46`, `glib 0.22.11`, `jsonwebtoken 10.4.0`,
  `tauri 2.13.0`, `imap 3.0.0-alpha.15`, `lexical-core` absent.
- `rustfmt --check` passes on touched Rust files.
- Full local Rust compilation is still blocked by the missing MSVC `link.exe`;
  the required PR status checks are the build source of truth.

## Follow-ups

- `issues/open/008-imap-stable-migration.md`
