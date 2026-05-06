import { load } from "@tauri-apps/plugin-store";
import { useState, useEffect, useCallback } from "react";

import { DEFAULT_SETTINGS } from "@/lib/settings";

import type { AppSettings } from "@/lib/settings";


const STORE_FILE = "settings.json";
const STORE_KEY = "app_settings";

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

    init();
    return () => { cancelled = true; };
  }, []);

  const update = useCallback(async (patch: Partial<AppSettings>) => {
    setSettings((prev) => {
      const next = { ...prev, ...patch };

      // Persist asynchronously — fire and forget
      load(STORE_FILE)
        .then((store) => store.set(STORE_KEY, next))
        .then(() => load(STORE_FILE).then((s) => s.save()))
        .catch(() => {});

      return next;
    });
  }, []);

  return { settings, loaded, update };
}
