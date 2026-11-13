# Desktop UI architecture

This folder contains the frontend entry point, document shell, styles, and the
inline SVG icon sprite. It is responsible for presentation and local preview
behavior, not direct SQLite access. Desktop builds call the Rust core through
`services/tauri.js`; the browser preview keeps a fallback workspace so the UI can
still be reviewed without Tauri.
