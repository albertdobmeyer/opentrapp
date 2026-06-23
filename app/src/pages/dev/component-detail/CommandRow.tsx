import { ChevronDown, ChevronRight, Loader2, Play, Square } from "lucide-react";
import { useState } from "react";



import { initialArgValues, nonEmpty } from "./helpers";
import { ArgFields, DangerPill, ResultPanel } from "./widgets";

import type { Command, CommandResult } from "@/lib/types";

import { runCommand, startStream, stopStream } from "@/lib/tauri";

/**
 * One manifest command: collapsible arg-form + Run (action/query) or Stream
 * (start/stop). Routes through the generic `run_command` / `start|stop_stream`
 * channel — no per-component knowledge.
 */
export function CommandRow({
  componentId,
  command,
}: {
  componentId: string;
  command: Command;
}) {
  const [open, setOpen] = useState(false);
  const [values, setValues] = useState<Record<string, string>>(() =>
    initialArgValues(command.args),
  );
  const [busy, setBusy] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [result, setResult] = useState<CommandResult | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const run = async () => {
    setBusy(true);
    setErr(null);
    setResult(null);
    try {
      setResult(await runCommand(componentId, command.id, nonEmpty(values)));
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const toggleStream = async () => {
    setErr(null);
    try {
      if (streaming) {
        await stopStream(componentId, command.id);
        setStreaming(false);
      } else {
        await startStream(componentId, command.id, nonEmpty(values));
        setStreaming(true);
      }
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    }
  };

  const isStream = command.type === "stream";

  return (
    <div className="rounded-md border border-neutral-800 bg-neutral-900/50 p-2">
      <div className="flex items-center justify-between gap-2">
        <button
          type="button"
          className="flex flex-1 items-center gap-1.5 text-left"
          onClick={() => {
            setOpen((o) => !o);
          }}
        >
          {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span className="text-sm text-neutral-200">{command.name}</span>
          <DangerPill danger={command.danger} />
          {command.tier === "advanced" && (
            <span className="pill pill-neutral">advanced</span>
          )}
        </button>
        {isStream ? (
          <button
            type="button"
            className={`btn btn-sm ${streaming ? "btn-danger" : "btn-ghost"}`}
            onClick={() => {
              void toggleStream();
            }}
          >
            {streaming ? <Square size={13} /> : <Play size={13} />}
            {streaming ? "Stop" : "Stream"}
          </button>
        ) : (
          <button
            type="button"
            className="btn btn-sm btn-primary"
            disabled={busy}
            onClick={() => {
              void run();
            }}
          >
            {busy ? (
              <Loader2 size={13} className="animate-spin" />
            ) : (
              <Play size={13} />
            )}
            Run
          </button>
        )}
      </div>

      {open && (
        <div className="mt-2 pl-5">
          {command.description && (
            <p className="text-xs text-neutral-500">{command.description}</p>
          )}
          <p className="mt-1 font-mono text-xs text-neutral-600">
            {command.command}
          </p>
          <ArgFields args={command.args} values={values} onChange={setValues} />
          {isStream && streaming && (
            <p className="mt-2 text-xs text-neutral-500">
              Streaming — output appears in the Logs view.
            </p>
          )}
        </div>
      )}

      {err && (
        <p className="mt-2 pl-5 font-mono text-xs text-danger-400">{err}</p>
      )}
      {result && (
        <div className="pl-5">
          <ResultPanel result={result} />
        </div>
      )}
    </div>
  );
}
