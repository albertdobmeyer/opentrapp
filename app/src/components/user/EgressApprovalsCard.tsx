import { Globe } from "lucide-react";
import { useEffect, useState } from "react";

import type { PendingApproval, AllowlistDecision } from "@/lib/types";

import { listEgressApprovals, applyAllowlistDecision } from "@/lib/tauri";


type State =
  | { kind: "loading" }
  | { kind: "unavailable" }
  | { kind: "ready"; pending: PendingApproval[] };

/**
 * Egress allowlist approvals (v0.6 Item A): the human-mediated loosening surface.
 * Your assistant's blocked attempts to reach a site it can't normally reach are
 * surfaced here, each with a plain-language reason from the on-device check. Only
 * you can approve one — the assistant never can (ADR-0002). Clear data-leak and
 * rebinding attempts are blocked outright and never appear here.
 *
 * "Allow always" carries a deliberate two-tap confirm (a small friction so a
 * loosening is never a single careless click — T5). "Block" only remembers the
 * choice; it never changes the safe list.
 */
export default function EgressApprovalsCard() {
  const [state, setState] = useState<State>({ kind: "loading" });
  const [confirming, setConfirming] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    const live = { current: true };
    void (async () => {
      try {
        const pending = await listEgressApprovals();
        if (live.current) setState({ kind: "ready", pending });
      } catch {
        if (live.current) setState({ kind: "unavailable" });
      }
    })();
    return () => {
      live.current = false;
    };
  }, []);

  async function decide(host: string, decision: AllowlistDecision) {
    setBusy(host);
    try {
      await applyAllowlistDecision(host, decision);
      setState((s) =>
        s.kind === "ready"
          ? { kind: "ready", pending: s.pending.filter((p) => p.host !== host) }
          : s,
      );
    } catch {
      // Leave the item in place; nothing destructive happened.
    } finally {
      setBusy(null);
      setConfirming(null);
    }
  }

  function onAllow(host: string) {
    // Deliberate friction (T5): the first tap arms, the second confirms.
    if (confirming === host) void decide(host, "always");
    else setConfirming(host);
  }

  return (
    <section className="mt-8 rounded-xl border border-neutral-800 bg-neutral-900/40 p-5">
      <div className="mb-1 flex items-center gap-2 text-neutral-200">
        <Globe size={18} strokeWidth={1.75} className="text-primary-400" />
        <h2 className="text-sm font-semibold">Sites waiting for your approval</h2>
      </div>
      <p className="mb-4 text-sm text-neutral-400">
        Your assistant tried to reach these sites, which aren&apos;t approved
        yet. Approve only the ones you recognise and trust.
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
          Nothing is waiting. If your assistant tries to reach a new site,
          you&apos;ll be asked here first.
        </p>
      )}

      {state.kind === "ready" && state.pending.length > 0 && (
        <ul className="space-y-3">
          {state.pending.map((p) => (
            <li
              key={p.host}
              className="rounded-lg border border-neutral-800 bg-neutral-900/60 p-4"
            >
              <div className="mb-1 text-sm font-medium text-neutral-100">
                {p.host}
              </div>
              <p className="mb-3 text-sm text-neutral-400">{p.reason}</p>
              <div className="flex gap-2">
                <button
                  type="button"
                  disabled={busy === p.host}
                  onClick={() => { onAllow(p.host); }}
                  className="rounded-md bg-primary-600/90 px-3 py-1.5 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50"
                >
                  {confirming === p.host ? "Tap again to confirm" : "Allow always"}
                </button>
                <button
                  type="button"
                  disabled={busy === p.host}
                  onClick={() => void decide(p.host, "deny")}
                  className="rounded-md border border-neutral-700 px-3 py-1.5 text-sm font-medium text-neutral-300 hover:bg-neutral-800 disabled:opacity-50"
                >
                  Block
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
