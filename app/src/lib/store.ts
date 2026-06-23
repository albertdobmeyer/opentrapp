/**
 * Persistent-store shim for the de-Tauri dual-mode GUI (ADR-0022): the Tauri store file in the
 * webview, a `localStorage`-backed store with the same shape in a plain browser. Its own module so
 * settings consumers (`useSettings`) import ONLY the store plugin — exactly the pre-de-Tauri import
 * graph, so no unrelated test needs to mock shell/clipboard. Returns the plugin's promise directly
 * (no extra microtask — timing-transparent vs. the raw plugin call).
 */
import { load as tauriLoad } from "@tauri-apps/plugin-store";

import { isTauriRuntime } from "./runtime";

/** The subset of the Tauri store API the app uses (best-effort key-value persistence). */
export interface KvStore {
  get<T>(key: string): Promise<T | undefined>;
  set(key: string, value: unknown): Promise<void>;
  save(): Promise<void>;
}

/**
 * Load a persistent store: the Tauri store file in the webview, a `localStorage`-backed store with
 * the same `get`/`set`/`save` shape in the browser. Keys are namespaced by file to avoid collisions.
 */
export function load(file: string): Promise<KvStore> {
  if (isTauriRuntime()) {
    return tauriLoad(file);
  }
  return Promise.resolve(new LocalStorageStore(file));
}

class LocalStorageStore implements KvStore {
  constructor(private readonly file: string) {}

  private key(name: string): string {
    return `otv-store:${this.file}:${name}`;
  }

  get<T>(name: string): Promise<T | undefined> {
    const raw = localStorage.getItem(this.key(name));
    return Promise.resolve(raw == null ? undefined : (JSON.parse(raw) as T));
  }

  set(name: string, value: unknown): Promise<void> {
    localStorage.setItem(this.key(name), JSON.stringify(value));
    return Promise.resolve();
  }

  // localStorage writes are synchronous + durable, so there is nothing to flush.
  save(): Promise<void> {
    return Promise.resolve();
  }
}
