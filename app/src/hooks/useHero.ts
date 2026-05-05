import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import {
  getAssistantStatus,
  type AssistantStatus,
  type AssistantStatusSnapshot,
} from "@/lib/tauri";

import { useSettings } from "./useSettings";

/**
 * The hero card on Home renders one of these states. This is a *user-facing*
 * state — derived from the backend `AssistantStatus` plus session context
 * (was the perimeter ever healthy this session? has the wizard run?).
 *
 * Mapping rules live in docs/specs/2026-04-30-pass-6-roadmap.md. As of
 * Pass 7 Day 2, `error_key` is reachable (backend probes Anthropic auth);
 * `paused_by_user` remains reserved for Day 4 (needs a "Pause" affordance
 * + intent flag).
 */
export type HeroState =
  | "not_setup"
  | "starting"
  | "running_safely"
  | "recovering"
  | "error_perimeter"
  | "error_key"
  | "paused_by_user";

export interface Hero {
  state: HeroState;
  /** Underlying backend snapshot — useful for the alerts banner + diagnostics. */
  snapshot: AssistantStatusSnapshot;
  /** True until the first evaluator tick lands. UI shows a brief skeleton. */
  loading: boolean;
}

const EMPTY_SNAPSHOT: AssistantStatusSnapshot = {
  status: "not_setup",
  alerts: [],
  last_checked_unix_ms: 0,
};

export function useHero(): Hero {
  const { settings } = useSettings();
  const [snapshot, setSnapshot] = useState<AssistantStatusSnapshot>(EMPTY_SNAPSHOT);
  const [loading, setLoading] = useState(true);
  const hasBeenRunningRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    (async () => {
      try {
        const initial = await getAssistantStatus();
        if (cancelled) return;
        if (initial.status === "ok") hasBeenRunningRef.current = true;
        setSnapshot(initial);
      } catch {
        // Tauri IPC unavailable (e.g. browser dev mode) — keep EMPTY_SNAPSHOT.
      } finally {
        if (!cancelled) setLoading(false);
      }

      unlisten = await listen<AssistantStatusSnapshot>(
        "assistant-status-changed",
        (event) => {
          if (event.payload.status === "ok") {
            hasBeenRunningRef.current = true;
          }
          setSnapshot(event.payload);
        },
      );
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const heroState = derive(
    snapshot.status,
    hasBeenRunningRef.current,
    settings.wizardCompleted,
  );

  return { state: heroState, snapshot, loading };
}

function derive(
  status: AssistantStatus,
  hasBeenRunning: boolean,
  wizardCompleted: boolean,
): HeroState {
  switch (status) {
    case "ok":
      return "running_safely";
    case "starting":
      return "starting";
    case "recovering":
      // First time we see a partial state, present it as "starting" — Karen
      // hasn't seen healthy yet, so "recovering" would be misleading copy.
      return hasBeenRunning ? "recovering" : "starting";
    case "error_perimeter":
      // If the wizard never ran, "perimeter stopped" actually means
      // "not yet set up", not "broken".
      return wizardCompleted ? "error_perimeter" : "not_setup";
    case "error_key":
      return "error_key";
    case "not_setup":
      return "not_setup";
    case "paused_by_user":
      return "paused_by_user";
  }
}
