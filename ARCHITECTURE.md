# Elyra architecture

Elyra is organized as a privacy-focused desktop client with a Tauri/Rust core,
local SQLite storage, shared packages, a Rust API server, and a static public
website. Mailbox behavior, cloud calls, diagnostics, cryptography, and secrets
live behind Rust modules. The webview is a presentation layer that talks to the
core through Tauri commands.

See `ai.md` for the full system diagram and architecture rules.
