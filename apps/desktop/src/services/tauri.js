export function invoke(command, args) {
  const tauri = window.__TAURI__;
  if (!tauri?.core?.invoke) {
    throw new Error("Tauri API is unavailable. Elyra must run inside the Tauri window.");
  }
  return tauri.core.invoke(command, args);
}
