import { load } from "@tauri-apps/plugin-store";
import { useState, useEffect, useCallback } from "react";

import type { AppSettings } from "@/lib/settings";

import { DEFAULT_SETTINGS } from "@/lib/settings";



const STORE_FILE = "settings.json";
const STORE_KEY = "app_settings";

/**
 * Best-effort persistence of settings to the Tauri store. A failure leaves
 * the in-memory state correct; the next successful write reconciles.
 */
async function persistSettings(next: AppSettings): Promise<void> {
  try {
    const store = await load(STORE_FILE);
    await store.set(STORE_KEY, next);
    await store.save();
  } catch {
    // Silently swallow — persistence is best-effort by design.
  }
}

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      try {
        const store = await load(STORE_FILE);
        const saved = await store.get<Partial<AppSettings>>(STORE_KEY);
        if (!cancelled) {
          // Merge saved with defaults for forward-compatibility
          setSettings({ ...DEFAULT_SETTINGS, ...saved });
          setLoaded(true);
        }
      } catch {
        // Store not available (e.g., during tests) — use defaults
        if (!cancelled) {
          setLoaded(true);
        }
      }
    }

    void init();
    return () => { cancelled = true; };
  }, []);

  // Returns Promise<void> so callers can `await update(...)`; persistence is
  // fire-and-forget so we resolve immediately. A failed disk write leaves the
  // in-memory state correct, and the next successful write reconciles.
  const update = useCallback((patch: Partial<AppSettings>): Promise<void> => {
    setSettings((prev) => {
      const next = { ...prev, ...patch };
      void persistSettings(next);
      return next;
    });
    return Promise.resolve();
  }, []);

  return { settings, loaded, update };
}
