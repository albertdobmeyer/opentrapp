import { ChevronDown, ChevronRight, Loader2, Play } from "lucide-react";
import { useMemo, useState } from "react";



import { initialArgValues, nonEmpty, type ArgLike } from "./helpers";
import { ArgFields, DangerPill } from "./widgets";

import type { Workflow, WorkflowInput } from "@/lib/types";

import { executeWorkflow } from "@/lib/tauri";

/**
 * One manifest workflow: collapsible inputs-form + Run via the generic
 * `execute_workflow` channel; shows the run's status + step tally. Workflow
 * inputs are projected onto the shared arg-form (url collapses to a text field).
 */
export function WorkflowRow({
  componentId,
  workflow,
}: {
  componentId: string;
  workflow: Workflow;
}) {
  const [open, setOpen] = useState(false);
  const inputsAsArgs: ArgLike[] = useMemo(
    () =>
      workflow.inputs.map((i: WorkflowInput) => ({
        id: i.id,
        name: i.label,
        type: i.type === "url" ? "string" : i.type,
        required: i.required,
        options: i.options,
        description: i.description,
        default: i.default,
      })),
    [workflow.inputs],
  );
  const [values, setValues] = useState<Record<string, string>>(() =>
    initialArgValues(inputsAsArgs),
  );
  const [busy, setBusy] = useState(false);
  const [summary, setSummary] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const run = async () => {
    setBusy(true);
    setErr(null);
    setSummary(null);
    try {
      const res = await executeWorkflow(
        componentId,
        workflow.id,
        nonEmpty(values),
      );
      const passed = res.steps.filter((s) => s.status === "passed").length;
      setSummary(
        `${res.status} · ${String(passed)}/${String(res.steps.length)} steps · ${String(res.duration_ms)} ms`,
      );
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const stepCount = workflow.steps.length;

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
          <span className="text-sm text-neutral-200">{workflow.name}</span>
          <DangerPill danger={workflow.danger} />
          <span className="pill pill-neutral">{workflow.trigger}</span>
        </button>
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
      </div>

      {open && (
        <div className="mt-2 pl-5">
          {workflow.description && (
            <p className="text-xs text-neutral-500">{workflow.description}</p>
          )}
          <p className="mt-1 text-xs text-neutral-600">
            {stepCount} step{stepCount === 1 ? "" : "s"}
          </p>
          <ArgFields args={inputsAsArgs} values={values} onChange={setValues} />
        </div>
      )}

      {summary && (
        <p className="mt-2 pl-5 text-xs text-neutral-300">{summary}</p>
      )}
      {err && (
        <p className="mt-2 pl-5 font-mono text-xs text-danger-400">{err}</p>
      )}
    </div>
  );
}
