import type { RuntimeAdapter } from "./adapter";
import { createHttpAdapter } from "./http-adapter";
import { createTauriAdapter } from "./tauri-adapter";

export function createRuntimeAdapter(): RuntimeAdapter {
  if (window.__TAURI_INTERNALS__) {
    return createTauriAdapter();
  }
  return createHttpAdapter();
}
