# UI assets architecture

This folder holds generated application icons used by the Tauri bundle. Interface
glyphs are not stored here: they live in the inline SVG sprite in `index.html` so
they inherit `currentColor` and theme tokens. Keep runtime assets independent of
Rust code.
