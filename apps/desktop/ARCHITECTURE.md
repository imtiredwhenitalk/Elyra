# Desktop architecture

The desktop app combines the HTML/CSS/JavaScript presentation layer with the Tauri Rust application core. UI state stays in `src`; native commands and local persistence stay in `src-tauri`.
