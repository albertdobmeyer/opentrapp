import { useEffect, useState } from "react";

import type { SentinelActivity } from "@/lib/types";

import { listen } from "@/lib/events";
import { getSentinelActivity } from "@/lib/tauri";


/**
 * The resting state: rung 0/1 only, no model running. Used as the initial
 * value and the fallback in browser mode (where the Tauri IPC is absent and
 * `getSentinelActivity` rejects — the indicator simply shows "watching").
 */
const WATCHING: SentinelActivity = {
  rung: "watching",
  label: "watching",
  since_unix_ms: 0,
};

/**
 * Subscribes to the backend `sentinel-activity-changed` event and exposes the
 * current Sentinel rung, so the Security page can show the user when their
 * machine is doing semantic judgment ("never wonder why it got busy" —
 * spec 01 §6). Mirrors the `useBootstrapProgress` event-hook pattern.
 */
export function useSentinelActivity(): SentinelActivity {
  const [activity, setActivity] = useState<SentinelActivity>(WATCHING);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    void (async () => {
      try {
        setActivity(await getSentinelActivity());
      } catch {
        // Browser mode — no Tauri IPC. Stay at the resting "watching" state.
      }
      unlisten = await listen<SentinelActivity>(
        "sentinel-activity-changed",
        (event) => { setActivity(event.payload); },
      );
    })();

    return () => unlisten?.();
  }, []);

  return activity;
}
