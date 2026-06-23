import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";


import { DangerPill } from "./widgets";

import type { Config } from "@/lib/types";

import { readConfig, writeConfig } from "@/lib/tauri";


/**
 * One manifest config: lazy-loaded on expand via `read_config`; editable configs
 * get a textarea + Save (`write_config`). Read-only configs render the content
 * without a Save affordance. Path-traversal is enforced backend-side
 * (commands/config.rs); this view only displays what the manifest declares.
 */
export function ConfigRow({
  componentId,
  config,
}: {
  componentId: string;
  config: Config;
}) {
  const [open, setOpen] = useState(false);
  const [content, setContent] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const openAndLoad = async () => {
    const next = !open;
    setOpen(next);
    if (!next || content !== null) return;
    setBusy(true);
    setErr(null);
    try {
      const text = await readConfig(componentId, config.path);
      setContent(text);
      setDraft(text);
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    setBusy(true);
    setErr(null);
    setSaved(false);
    try {
      await writeConfig(componentId, config.path, draft);
      setContent(draft);
      setSaved(true);
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="rounded-md border border-neutral-800 bg-neutral-900/50 p-2">
      <button
        type="button"
        className="flex w-full items-center gap-1.5 text-left"
        onClick={() => {
          void openAndLoad();
        }}
      >
        {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <span className="font-mono text-xs text-neutral-300">{config.path}</span>
        <span className="pill pill-neutral">{config.format}</span>
        {config.editable ? (
          <DangerPill danger={config.danger} />
        ) : (
          <span className="pill pill-neutral">read-only</span>
        )}
      </button>

      {open && (
        <div className="mt-2 pl-5">
          {config.description && (
            <p className="mb-2 text-xs text-neutral-500">{config.description}</p>
          )}
          {busy && content === null ? (
            <p className="text-xs text-neutral-500">Loading…</p>
          ) : (
            <>
              <textarea
                className="input min-h-[8rem] w-full font-mono text-xs"
                value={draft}
                readOnly={!config.editable}
                onChange={(e) => {
                  setDraft(e.target.value);
                  setSaved(false);
                }}
              />
              {config.editable && (
                <div className="mt-2 flex items-center gap-2">
                  <button
                    type="button"
                    className="btn btn-sm btn-primary"
                    disabled={busy || draft === content}
                    onClick={() => {
                      void save();
                    }}
                  >
                    Save
                  </button>
                  {config.restart_required && (
                    <span className="text-xs text-warning-400">
                      restart required to apply
                    </span>
                  )}
                  {saved && (
                    <span className="text-xs text-success-400">saved</span>
                  )}
                </div>
              )}
            </>
          )}
          {err && (
            <p className="mt-2 font-mono text-xs text-danger-400">{err}</p>
          )}
        </div>
      )}
    </div>
  );
}
