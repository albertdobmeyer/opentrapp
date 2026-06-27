import { ShieldAlert } from "lucide-react";
import { useEffect, useState } from "react";

import type { PendingWeakening } from "@/lib/types";

import { listPendingApprovals, approveWeakening } from "@/lib/tauri";


type State =
  | { kind: "loading" }
  | { kind: "unavailable" }
  | { kind: "ready"; pending: PendingWeakening[] };

/** Friendly, assistant-first copy for each held action (no developer vocabulary, CLAUDE.md §3). */
function describe(verb: string): { title: string; detail: string } {
  switch (verb) {
    case "pause":
      return {
        title: "Pause your assistant",
        detail:
          "Your assistant will stop running until you start it again. It stays protected.",
      };
    case "shutdown":
      return {
        title: "Stop your assistant",
        detail: "Your assistant and its protection will shut down completely.",
      };
    default:
      return {
        title: "Approve this action",
        detail: "This action reduces your protection.",
      };
  }
}

/**
 * Actions waiting for your approval (ADR-0021). When something asks to reduce your
 * protection — pausing or stopping your assistant — it is HELD here for your explicit
 * OK. Your assistant can request it but can never approve it; only you can, on this
 * surface (which it cannot reach), with a deliberate two-tap confirm (a small friction
 * so a reduction is never a single careless click).
 */
export default function WeakeningApprovalsCard() {
  const [state, setState] = useState<State>({ kind: "loading" });
  const [confirming, setConfirming] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    const live = { current: true };
    void (async () => {
      try {
        const pending = await listPendingApprovals();
        if (live.current) setState({ kind: "ready", pending });
      } catch {
        if (live.current) setState({ kind: "unavailable" });
      }
    })();
    return () => {
      live.current = false;
    };
  }, []);

  async function approve(id: string) {
    setBusy(id);
    try {
      await approveWeakening(id);
      setState((s) =>
        s.kind === "ready"
          ? { kind: "ready", pending: s.pending.filter((p) => p.id !== id) }
          : s,
      );
    } catch {
      // Leave it in place; nothing was applied.
    } finally {
      setBusy(null);
      setConfirming(null);
    }
  }

  function onApprove(id: string) {
    // Deliberate friction: the first tap arms, the second confirms.
    if (confirming === id) void approve(id);
    else setConfirming(id);
  }

  return (
    <section className="mt-8 rounded-xl border border-neutral-800 bg-neutral-900/40 p-5">
      <div className="mb-1 flex items-center gap-2 text-neutral-200">
        <ShieldAlert size={18} strokeWidth={1.75} className="text-amber-400" />
        <h2 className="text-sm font-semibold">Actions waiting for your approval</h2>
      </div>
      <p className="mb-4 text-sm text-neutral-400">
        Your assistant asked to do something that reduces your protection. Only you
        can approve it — your assistant never can.
      </p>

      {state.kind === "loading" && (
        <p className="text-sm text-neutral-500">Checking…</p>
      )}

      {state.kind === "unavailable" && (
        <p className="text-sm text-neutral-500">
          This becomes available once your assistant is running.
        </p>
      )}

      {state.kind === "ready" && state.pending.length === 0 && (
        <p className="text-sm text-neutral-500">
          Nothing is waiting for your approval.
        </p>
      )}

      {state.kind === "ready" && state.pending.length > 0 && (
        <ul className="space-y-3">
          {state.pending.map((p) => {
            const { title, detail } = describe(p.verb);
            return (
              <li
                key={p.id}
                className="rounded-lg border border-neutral-800 bg-neutral-900/60 p-4"
              >
                <div className="mb-1 text-sm font-medium text-neutral-100">
                  {title}
                </div>
                <p className="mb-3 text-sm text-neutral-400">{detail}</p>
                <button
                  type="button"
                  disabled={busy === p.id}
                  onClick={() => {
                    onApprove(p.id);
                  }}
                  className="rounded-md bg-amber-600/90 px-3 py-1.5 text-sm font-medium text-white hover:bg-amber-600 disabled:opacity-50"
                >
                  {confirming === p.id ? "Tap again to confirm" : "Approve"}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
