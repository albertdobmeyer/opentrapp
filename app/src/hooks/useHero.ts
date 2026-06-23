import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import { useSettings } from "./useSettings";

import {
  getAssistantStatus,
  type AssistantStatus,
  type AssistantStatusSnapshot,
} from "@/lib/tauri";


/**
 * The hero card on Home renders one of these states. This is a *user-facing*
 * state — derived from the backend `AssistantStatus` plus session context.
 *
 * v0.4 adds four new values for the two-axis bootstrap×tenant model:
 * - installing / bootstrapping — first-launch setup in progress
 * - shell_ready_absent — shell up, user hasn't activated yet
 * - shell_failed — shell setup failed; recovery card shown
 */
export type HeroState =
  | "installing"
  | "bootstrapping"
  | "shell_ready_absent"
  | "shell_failed"
  | "not_setup"
  | "starting"
  | "running_safely"
  | "recovering"
  | "error_perimeter"
  | "error_key"
  | "paused_by_user"
  | "dormant";

export interface Hero {
  state: HeroState;
  /** Underlying backend snapshot — useful for the alerts banner + diagnostics. */
  snapshot: AssistantStatusSnapshot;
  /** True until the first evaluator tick lands. UI shows a brief skeleton. */
  loading: boolean;
}

const EMPTY_SNAPSHOT: AssistantStatusSnapshot = {
  status: "installing",
  alerts: [],
  last_checked_unix_ms: 0,
  bootstrap_failure: null,
};

export function useHero(): Hero {
  const { settings } = useSettings();
  const [snapshot, setSnapshot] = useState<AssistantStatusSnapshot>(EMPTY_SNAPSHOT);
  const [loading, setLoading] = useState(true);
  const hasBeenRunningRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void (async () => {
      try {
        const initial = await getAssistantStatus();
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- closure-mutated by cleanup function; ESLint's narrowing is unaware
        if (cancelled) return;
        if (initial.status === "ok") hasBeenRunningRef.current = true;
        setSnapshot(initial);
      } catch {
        // Tauri IPC unavailable (e.g. browser dev mode) — keep EMPTY_SNAPSHOT.
      } finally {
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- closure-mutated by cleanup function; ESLint's narrowing is unaware
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
    case "installing":
      return "installing";
    case "bootstrapping":
      return "bootstrapping";
    case "shell_ready_absent":
      return "shell_ready_absent";
    case "shell_failed":
      return "shell_failed";
    case "ok":
      return "running_safely";
    case "starting":
      return "starting";
    case "recovering":
      // First time we see a partial state, present it as "starting" — the user
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
    case "dormant":
      return "dormant";
  }
}
