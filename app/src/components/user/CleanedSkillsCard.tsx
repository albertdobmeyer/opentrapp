import { ScanSearch } from "lucide-react";
import { useEffect, useState } from "react";

import { runCommand } from "@/lib/tauri";

interface CleanedSkill {
  skill: string;
  report: string;
}

type State =
  | { kind: "loading" }
  | { kind: "unavailable" }
  | { kind: "ready"; cleaned: CleanedSkill[] };

/**
 * The disarm-diff display (v0.6): a read-only trust artifact showing what the
 * security cleanroom removed from skills it processed. Data flows through the
 * generic manifest channel — `run_command("skills","cleaned-skills")` runs the
 * disarm-report inside the skills container (where the delivered skills + their
 * DISARM-DIFF.txt live) and returns plain-language JSON. The host never reads a
 * container volume directly; the untrusted agent never feeds this a path.
 *
 * Degrades honestly: when the assistant isn't running, the command can't reach
 * the container, so the card says so rather than pretending there's nothing.
 */
export default function CleanedSkillsCard() {
  const [state, setState] = useState<State>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const res = await runCommand("skills", "cleaned-skills");
        if (cancelled) return;
        if (res.exit_code !== 0) {
          setState({ kind: "unavailable" });
          return;
        }
        const parsed = JSON.parse(res.stdout) as { cleaned?: CleanedSkill[] };
        setState({ kind: "ready", cleaned: parsed.cleaned ?? [] });
      } catch {
        if (!cancelled) setState({ kind: "unavailable" });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <section className="mt-8 rounded-xl border border-neutral-800 bg-neutral-900/40 p-5">
      <div className="mb-1 flex items-center gap-2 text-neutral-200">
        <ScanSearch size={18} strokeWidth={1.75} className="text-primary-400" />
        <h2 className="text-sm font-semibold">What was removed from your skills</h2>
      </div>
      <p className="mb-4 text-sm text-neutral-400">
        Before a skill is installed, it's rebuilt from scratch and anything
        unsafe is dropped. Here's exactly what was taken out, in plain language.
      </p>

      {state.kind === "loading" && (
        <p className="text-sm text-neutral-500">Checking…</p>
      )}

      {state.kind === "unavailable" && (
        <p className="text-sm text-neutral-500">
          This becomes available once your assistant is running.
        </p>
      )}

      {state.kind === "ready" && state.cleaned.length === 0 && (
        <p className="text-sm text-neutral-500">
          No skills have needed cleaning yet. When one does, you'll see here
          what was removed.
        </p>
      )}

      {state.kind === "ready" && state.cleaned.length > 0 && (
        <ul className="space-y-3">
          {state.cleaned.map((c) => (
            <li
              key={c.skill}
              className="rounded-lg border border-neutral-800 bg-neutral-900/60 p-4"
            >
              <div className="mb-1 text-sm font-medium text-neutral-100">
                {c.skill}
              </div>
              <pre className="whitespace-pre-wrap break-words font-sans text-sm text-neutral-400">
                {c.report}
              </pre>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
