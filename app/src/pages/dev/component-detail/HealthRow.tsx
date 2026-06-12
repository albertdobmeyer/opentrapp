import { Loader2, RefreshCw } from "lucide-react";
import { useState } from "react";

import { runHealthProbe } from "@/lib/tauri";

import type { HealthProbe } from "@/lib/types";

/**
 * One manifest health probe: a Check button runs the probe via the generic
 * `run_health_probe` channel and shows pass/fail + raw output.
 */
export function HealthRow({
  componentId,
  probe,
}: {
  componentId: string;
  probe: HealthProbe;
}) {
  const [busy, setBusy] = useState(false);
  const [out, setOut] = useState<{ exit_code: number; stdout: string } | null>(
    null,
  );
  const [err, setErr] = useState<string | null>(null);

  const run = async () => {
    setBusy(true);
    setErr(null);
    try {
      const res = await runHealthProbe(
        componentId,
        probe.command,
        probe.timeout_seconds,
      );
      setOut({ exit_code: res.exit_code, stdout: res.stdout });
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="rounded-md border border-neutral-800 bg-neutral-900/50 p-2">
      <div className="flex items-center justify-between gap-2">
        <div>
          <span className="text-sm text-neutral-200">{probe.name}</span>
          {out && (
            <span
              className={`pill ml-2 ${
                out.exit_code === 0 ? "pill-safe" : "pill-danger"
              }`}
            >
              {out.exit_code === 0 ? "ok" : `exit ${String(out.exit_code)}`}
            </span>
          )}
        </div>
        <button
          type="button"
          className="btn btn-sm btn-ghost"
          disabled={busy}
          onClick={() => {
            void run();
          }}
        >
          {busy ? (
            <Loader2 size={13} className="animate-spin" />
          ) : (
            <RefreshCw size={13} />
          )}
          Check
        </button>
      </div>
      {out?.stdout && (
        <pre className="mt-2 max-h-24 overflow-auto whitespace-pre-wrap font-mono text-xs text-neutral-400">
          {out.stdout}
        </pre>
      )}
      {err && <p className="mt-2 font-mono text-xs text-danger-400">{err}</p>}
    </div>
  );
}
