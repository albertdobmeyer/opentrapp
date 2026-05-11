import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import {
  getAssistantStatus,
  type AssistantStatusSnapshot,
  type BackendAlert,
} from "@/lib/tauri";

import { useSettings } from "./useSettings";

export type AlertSeverity = "danger" | "warning" | "info";

export interface Alert {
  /** Stable identifier — used for dismissal persistence. */
  id: string;
  severity: AlertSeverity;
  title: string;
  /** Optional secondary explanation. */
  body?: string;
  /** Optional CTA. `to` is a router path. */
  cta?: { label: string; to: string };
  dismissable?: boolean;
}

const EMPTY_SNAPSHOT: AssistantStatusSnapshot = {
  status: "not_setup",
  alerts: [],
  last_checked_unix_ms: 0,
  bootstrap_failure: null,
};

/**
 * Backend-driven alerts (Pass 7 Day 2). Subscribes to the
 * `assistant-status-changed` event emitted by `status_aggregator.rs`'s
 * 60s evaluator. The four rules — missing-anthropic-key,
 * invalid-anthropic-key, missing-telegram-token, perimeter-error —
 * all live in Rust now (`build_alerts`).
 *
 * The frontend's only filter responsibility is suppressing alerts
 * during the wizard (set via the alert's `suppress_during_wizard`
 * flag) and applying the user's persisted dismissals.
 *
 * Spending-limit alerts were dropped from scope per the 2026-05-02
 * vision recheck — Anthropic Console handles billing.
 */
export function useAlerts(): { alerts: Alert[]; dismiss: (id: string) => void } {
  const { settings, update } = useSettings();
  const [snapshot, setSnapshot] = useState<AssistantStatusSnapshot>(EMPTY_SNAPSHOT);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void (async () => {
      try {
        const initial = await getAssistantStatus();
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- closure-mutated by cleanup function below; ESLint's narrowing is unaware
        if (!cancelled) setSnapshot(initial);
      } catch {
        // Tauri IPC unavailable (e.g. browser dev mode) — keep EMPTY_SNAPSHOT.
      }

      unlisten = await listen<AssistantStatusSnapshot>(
        "assistant-status-changed",
        (event) => {
          setSnapshot(event.payload);
        },
      );
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const alerts = snapshot.alerts
    .filter((a) => !(a.suppress_during_wizard && !settings.wizardCompleted))
    .filter((a) => !settings.dismissedAlerts[a.id])
    .map((a) => toFrontendAlert(a));

  function dismiss(id: string) {
    void update({
      dismissedAlerts: { ...settings.dismissedAlerts, [id]: Date.now() },
    });
  }

  return { alerts, dismiss };
}

function toFrontendAlert(a: BackendAlert): Alert {
  return {
    id: a.id,
    severity: a.severity,
    title: a.title,
    body: a.body ?? undefined,
    cta:
      a.cta_label && a.cta_to
        ? { label: a.cta_label, to: a.cta_to }
        : undefined,
    dismissable: a.dismissable,
  };
}
