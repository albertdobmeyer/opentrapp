import { listen } from "@tauri-apps/api/event";
import {
  Check,
  ChevronDown,
  ChevronRight,
  Circle,
  ExternalLink,
  Loader2,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";


import ContactSupport from "@/components/failure/ContactSupport";
import FriendlyRetry from "@/components/failure/FriendlyRetry";
import { useSettings } from "@/hooks/useSettings";
import { classifyError, type ClassifiedError } from "@/lib/errors";
import {
  checkPrerequisites,
  deriveTelegramBotUrl,
  executeWorkflow,
  initSubmodules,
  readConfig,
  startStream,
  stopStream,
} from "@/lib/tauri";
import { parseEnvKeys, withRetry } from "@/lib/wizardUtils";

import type { StreamEnd, StreamLine } from "@/lib/types";

type SubStepId = "check" | "download" | "build" | "safety";
type SubStepStatus = "pending" | "running" | "succeeded" | "failed";

interface SubStep {
  id: SubStepId;
  label: string;
  status: SubStepStatus;
  startedAt: number | null;
  durationMs: number | null;
  retryAttempt: number;
  technicalLog: string[];
}

const INITIAL_STEPS: SubStep[] = [
  { id: "check", label: "Check your computer", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
  { id: "download", label: "Download the AI parts", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
  { id: "build", label: "Build your assistant", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
  { id: "safety", label: "Test safety checks", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
];

interface Props {
  onComplete: () => void;
  onBack: () => void;
}

type Outcome =
  | { kind: "running" }
  | { kind: "missing-runtime" }
  | { kind: "failed"; classified: ClassifiedError; level: 2 | 3 }
  | { kind: "succeeded" };

/** Redact API keys and tokens from streamed build output. */
function sanitizeLine(text: string): string {
  return text
    .replace(/(ANTHROPIC_API_KEY=|sk-ant-api\d{2}-)[^\s"']+/g, "$1[REDACTED]")
    .replace(/(OPENAI_API_KEY=|sk-)[\w-]{10,}/g, "$1[REDACTED]")
    .replace(/(TELEGRAM_BOT_TOKEN=)\S+/g, "$1[REDACTED]")
    .replace(/(-e\s+ANTHROPIC_API_KEY=)\S+/g, "$1[REDACTED]")
    .replace(/(-e\s+TELEGRAM_BOT_TOKEN=)\S+/g, "$1[REDACTED]");
}

export default function InstallStep({ onComplete, onBack }: Props) {
  const { update } = useSettings();
  const [steps, setSteps] = useState<SubStep[]>(INITIAL_STEPS);
  const [outcome, setOutcome] = useState<Outcome>({ kind: "running" });
  const [showDetails, setShowDetails] = useState(false);
  const [tick, setTick] = useState(0);
  const unlistenersRef = useRef<(() => void)[]>([]);
  const cancelledRef = useRef(false);
  const currentSubStepRef = useRef<SubStepId | null>(null);

  const updateStep = useCallback(
    (id: SubStepId, patch: Partial<SubStep>) => {
      setSteps((prev) =>
        prev.map((s) => (s.id === id ? { ...s, ...patch } : s)),
      );
    },
    [],
  );

  const appendLog = useCallback((id: SubStepId, line: string) => {
    setSteps((prev) =>
      prev.map((s) =>
        s.id === id ? { ...s, technicalLog: [...s.technicalLog, line] } : s,
      ),
    );
  }, []);

  // Live timer refresh for the running step.
  useEffect(() => {
    const interval = setInterval(() => { setTick((t) => t + 1); }, 1000);
    return () => { clearInterval(interval); };
  }, []);

  // Cleanup stream listeners on unmount.
  useEffect(() => {
    return () => {
      cancelledRef.current = true;
      for (const fn of unlistenersRef.current) fn();
    };
  }, []);

  /** Stream a command to completion. Resolves on exit_code 0, rejects otherwise. */
  const streamOneCommand = useCallback(
    async (
      componentId: string,
      commandId: string,
      subStepId: SubStepId,
    ): Promise<void> => {
      return new Promise(async (resolve, reject) => {
        let settled = false;
        const unlistenLine = await listen<StreamLine>("stream-line", (event) => {
          if (
            event.payload.component_id !== componentId ||
            event.payload.command_id !== commandId
          ) {
            return;
          }
          appendLog(subStepId, sanitizeLine(event.payload.line));
        });
        const unlistenEnd = await listen<StreamEnd>("stream-end", (event) => {
          if (
            event.payload.component_id !== componentId ||
            event.payload.command_id !== commandId
          ) {
            return;
          }
          if (settled) return;
          settled = true;
          unlistenLine();
          unlistenEnd();
          if (event.payload.exit_code === 0) {
            resolve();
          } else {
            reject(
              new Error(
                `${componentId} ${commandId} exited with code ${event.payload.exit_code}`,
              ),
            );
          }
        });
        unlistenersRef.current.push(unlistenLine, unlistenEnd);

        try {
          await startStream(componentId, commandId);
        } catch (error) {
          if (!settled) {
            settled = true;
            unlistenLine();
            unlistenEnd();
            reject(error);
          }
        }
      });
    },
    [appendLog],
  );

  const runPipeline = useCallback(async () => {
    cancelledRef.current = false;
    setOutcome({ kind: "running" });
    setSteps(INITIAL_STEPS);

    try {
      // ── A: Check your computer ─────────────────────────────────────────
      currentSubStepRef.current = "check";
      updateStep("check", { status: "running", startedAt: Date.now() });
      const prereqReport = await checkPrerequisites();
      appendLog(
        "check",
        `Sandbox runner: ${prereqReport.container_runtime.found ? "ready" : "not found"}`,
      );
      if (!prereqReport.container_runtime.found) {
        // User-action required — do not auto-retry or escalate to FriendlyRetry.
        updateStep("check", {
          status: "failed",
          durationMs: 0,
        });
        setOutcome({ kind: "missing-runtime" });
        return;
      }
      updateStep("check", { status: "succeeded" });

      // ── B: Download the AI parts (submodules) ──────────────────────────
      currentSubStepRef.current = "download";
      updateStep("download", { status: "running", startedAt: Date.now() });
      await withRetry(
        async () => {
          appendLog("download", "Downloading your assistant…");
          const out = await initSubmodules();
          if (out) appendLog("download", out);
        },
        2,
        (attempt) => { updateStep("download", { retryAttempt: attempt }); },
      );
      // Re-check to pick up newly cloned submodules.
      const postInitReport = await checkPrerequisites();
      if (
        !postInitReport.submodules.every((s) => s.cloned && s.has_manifest)
      ) {
        throw new Error("Some assistant modules failed to download");
      }
      updateStep("download", { status: "succeeded" });

      // ── C: Build your assistant (stream vault + forge; skip pioneer) ──
      currentSubStepRef.current = "build";
      updateStep("build", { status: "running", startedAt: Date.now() });
      const components = new Set(postInitReport.components
        .map((c) => c.component_id)
        .filter((id) => id !== "moltbook-pioneer"));

      // Build vault first (setup + start), then forge (setup). Each streams
      // to the shared technical log for the build sub-step. Serialised
      // because forge's setup depends on vault networking being in place
      // (matches first-run-setup workflow's depends_on wiring).
      if (components.has("openclaw-vault")) {
        await withRetry(
          async () => {
            appendLog("build", "→ Your assistant: install");
            await streamOneCommand("openclaw-vault", "setup", "build");
            appendLog("build", "→ Your assistant: start");
            await streamOneCommand("openclaw-vault", "start", "build");
          },
          2,
          (attempt) =>
            { updateStep("build", { retryAttempt: attempt }); },
        );
      }
      if (components.has("clawhub-forge")) {
        await withRetry(
          async () => {
            appendLog("build", "→ Skill scanner: install");
            await streamOneCommand("clawhub-forge", "setup", "build");
          },
          2,
          (attempt) => { updateStep("build", { retryAttempt: attempt }); },
        );
      }
      updateStep("build", { status: "succeeded" });

      // ── D: Test safety checks (parallel: vault 24-point + forge pipeline)─
      currentSubStepRef.current = "safety";
      updateStep("safety", { status: "running", startedAt: Date.now() });
      await withRetry(
        async () => {
          const tasks: Promise<unknown>[] = [];
          if (components.has("openclaw-vault")) {
            appendLog("safety", "Running assistant security audit (24 checks)…");
            tasks.push(executeWorkflow("openclaw-vault", "full-verify"));
          }
          if (components.has("clawhub-forge")) {
            appendLog("safety", "Running skill scanner pipeline check…");
            tasks.push(executeWorkflow("clawhub-forge", "full-check"));
          }
          const results = await Promise.all(tasks);
          // WorkflowResult.status must be "completed" for a pass.
          for (const r of results as { status: string }[]) {
            if (r.status !== "completed") {
              throw new Error(`Workflow ended with status: ${r.status}`);
            }
          }
        },
        2,
        (attempt) => { updateStep("safety", { retryAttempt: attempt }); },
      );
      updateStep("safety", { status: "succeeded" });

      // ── Telegram prefetch — best-effort, do not block completion ──────
      void prefetchTelegramUrl(update);

      setOutcome({ kind: "succeeded" });
    } catch (error) {
      if (cancelledRef.current) return;
      const classified = classifyError(error, currentSubStepRef.current ?? undefined);
      // Mark any running step as failed.
      setSteps((prev) =>
        prev.map((s) =>
          s.status === "running" ? { ...s, status: "failed" } : s,
        ),
      );
      setOutcome({
        kind: "failed",
        classified,
        level: classified.retryable ? 2 : 3,
      });
    }
  }, [appendLog, streamOneCommand, update, updateStep]);

  // Kick off pipeline on mount.
  useEffect(() => {
    runPipeline();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Stop any in-flight stream on unmount.
  useEffect(() => {
    return () => {
      // Best-effort; ignore errors if nothing's actually streaming.
      void stopStream("openclaw-vault", "setup").catch(() => {});
      void stopStream("openclaw-vault", "start").catch(() => {});
      void stopStream("clawhub-forge", "setup").catch(() => {});
    };
  }, []);

  // Auto-advance once succeeded, after a brief pause so the user can see the
  // final green state.
  useEffect(() => {
    if (outcome.kind !== "succeeded") return;
    const t = setTimeout(() => { onComplete(); }, 1000);
    return () => { clearTimeout(t); };
  }, [outcome, onComplete]);

  // Guard against user triggering a click while the retry is pending.
  const handleRetry = () => {
    for (const fn of unlistenersRef.current) fn();
    unlistenersRef.current = [];
    runPipeline();
  };

  // ── Render branches ─────────────────────────────────────────────────────

  if (outcome.kind === "missing-runtime") {
    return <MissingRuntimeCard onBack={onBack} onRetry={handleRetry} />;
  }

  if (outcome.kind === "failed" && outcome.level === 3) {
    return (
      <ContactSupport
        classified={outcome.classified}
        onRetry={handleRetry}
        titleOverride="Setup couldn't finish"
      />
    );
  }

  if (outcome.kind === "failed" && outcome.level === 2) {
    return (
      <FriendlyRetry
        classified={outcome.classified}
        onRetry={handleRetry}
        secondary={{ label: "Go back", action: onBack }}
        onGetHelp={() =>
          { setOutcome({
            kind: "failed",
            classified: outcome.classified,
            level: 3,
          }); }
        }
      />
    );
  }

  // ── Running / Succeeded: show checklist ─────────────────────────────────

  const runningStep = steps.find((s) => s.status === "running");
  const totalRemainingMs = estimateRemaining(steps);

  return (
    <div className="animate-fade-in mx-auto max-w-xl px-4 py-6">
      <div className="mb-8 flex flex-col items-center text-center">
        <PulsingRings className="mb-6" />
        <h1 className="mb-2 text-2xl font-semibold text-neutral-100">
          Setting up your assistant
        </h1>
        <p className="text-sm text-neutral-400" aria-live="polite">
          {outcome.kind === "succeeded"
            ? "All done. Taking you to the finish line…"
            : (runningStep
              ? `${runningStep.label}…`
              : "This usually takes 2–3 minutes.")}
        </p>
        {totalRemainingMs !== null && outcome.kind === "running" && (
          <p className="mt-1 text-xs text-neutral-500">
            About {Math.max(1, Math.round(totalRemainingMs / 60000))} minute
            {totalRemainingMs >= 120000 ? "s" : ""} remaining
          </p>
        )}
      </div>

      <ul className="card-raised mb-6 space-y-3">
        {steps.map((step) => {
          const elapsed = step.startedAt
            ? (step.durationMs ?? Date.now() - step.startedAt)
            : null;
          return (
            <li key={step.id} className="flex items-center gap-3">
              <StepGlyph status={step.status} />
              <div className="flex-1">
                <p
                  className={`text-sm ${
                    step.status === "succeeded"
                      ? "text-neutral-300"
                      : step.status === "running"
                        ? "text-neutral-100"
                        : step.status === "failed"
                          ? "text-danger-400"
                          : "text-neutral-500"
                  }`}
                >
                  {step.label}
                </p>
              </div>
              {step.status === "running" && elapsed !== null && (
                <span
                  key={`elapsed-${tick}`}
                  className="text-xs tabular-nums text-neutral-500"
                >
                  {formatElapsed(elapsed)}
                </span>
              )}
            </li>
          );
        })}
      </ul>

      <div className="text-center">
        <button
          type="button"
          onClick={() => { setShowDetails((v) => !v); }}
          className="inline-flex items-center gap-1 text-xs text-neutral-500 hover:text-neutral-300"
          aria-expanded={showDetails}
        >
          {showDetails ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          {showDetails ? "Hide" : "Show"} technical details
        </button>
      </div>

      {showDetails && (
        <pre className="mt-4 max-h-64 overflow-y-auto rounded-md bg-neutral-950 p-3 font-mono text-[11px] leading-relaxed text-neutral-400 whitespace-pre-wrap break-all">
          {steps
            .flatMap((s) =>
              s.technicalLog.length > 0
                ? [`─── ${s.label} ───`, ...s.technicalLog, ""]
                : [],
            )
            .join("\n") || "(no output yet)"}
        </pre>
      )}
    </div>
  );
}

// ─── Sub-components ─────────────────────────────────────────────────────────

function StepGlyph({ status }: { status: SubStepStatus }) {
  switch (status) {
    case "pending":
      return <Circle size={18} className="text-neutral-700" strokeWidth={1.5} />;
    case "running":
      return <Loader2 size={18} className="animate-spin text-primary-400" />;
    case "succeeded":
      return <Check size={18} className="text-success-400" />;
    case "failed":
      return (
        <Circle
          size={18}
          strokeWidth={3}
          className="text-danger-400"
          fill="currentColor"
        />
      );
  }
}

function PulsingRings({ className }: { className?: string }) {
  return (
    <div
      className={`relative h-20 w-20 ${className ?? ""}`}
      role="img"
      aria-label="Installation in progress"
    >
      <span
        aria-hidden
        className="animate-pulse-ring absolute inset-0 rounded-full border-2 border-primary-500/40"
      />
      <span
        aria-hidden
        className="animate-pulse-ring absolute inset-3 rounded-full border-2 border-primary-500/60"
        style={{ animationDelay: "0.3s" }}
      />
      <span
        aria-hidden
        className="absolute inset-7 rounded-full bg-primary-500"
      />
    </div>
  );
}

function MissingRuntimeCard({
  onBack,
  onRetry,
}: {
  onBack: () => void;
  onRetry: () => void;
}) {
  const platform = detectPlatform();
  return (
    <div className="mx-auto max-w-xl px-4 py-10 animate-fade-in">
      <div className="card-raised">
        <h2 className="mb-2 text-xl font-semibold text-neutral-100">
          One thing missing
        </h2>
        <p className="mb-4 text-sm text-neutral-400">
          You'll need a sandbox runner installed first. It's free, takes a
          minute, and is what keeps your assistant safely separated from the
          rest of your computer.
        </p>

        <div className="mb-5 space-y-2">
          {platform === "linux" && (
            <InstallLine
              title="Linux (Ubuntu/Debian)"
              body="Open the guide for a one-click installer."
              advancedCode="sudo apt install podman podman-compose"
              href="https://podman.io/docs/installation#installing-on-linux"
            />
          )}
          {platform === "mac" && (
            <InstallLine
              title="macOS"
              body="Download Podman Desktop and run the installer."
              href="https://podman-desktop.io/downloads"
            />
          )}
          {platform === "windows" && (
            <InstallLine
              title="Windows"
              body="Download Podman Desktop and run the installer."
              href="https://podman-desktop.io/downloads"
            />
          )}
          {platform === "other" && (
            <InstallLine
              title="Install a sandbox runner"
              body="Choose the option for your computer:"
              href="https://podman-desktop.io/downloads"
            />
          )}
        </div>

        <p className="mb-6 text-xs text-neutral-500">
          After installing, click "Check again" — we'll pick it up
          automatically.
        </p>

        <div className="flex items-center justify-between">
          <button type="button" onClick={onBack} className="btn btn-md btn-ghost">
            Back
          </button>
          <button type="button" onClick={onRetry} className="btn btn-md btn-primary">
            Check again
          </button>
        </div>
      </div>
    </div>
  );
}

function InstallLine({
  title,
  body,
  advancedCode,
  href,
}: {
  title: string;
  body: string;
  /** Optional terminal command — hidden by default behind a disclosure. */
  advancedCode?: string;
  href: string;
}) {
  const [showAdvanced, setShowAdvanced] = useState(false);
  return (
    <div className="rounded-md bg-neutral-900 p-3">
      <p className="text-sm font-medium text-neutral-100">{title}</p>
      <p className="mt-1 text-xs text-neutral-400">{body}</p>
      <a
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        className="mt-2 inline-flex items-center gap-1 text-xs text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
      >
        Open guide <ExternalLink size={10} />
      </a>
      {advancedCode && (
        <div className="mt-3 border-t border-neutral-800 pt-2">
          <button
            type="button"
            onClick={() => { setShowAdvanced((v) => !v); }}
            className="inline-flex items-center gap-1 text-[11px] text-neutral-500 hover:text-neutral-300"
            aria-expanded={showAdvanced}
          >
            {showAdvanced ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
            {showAdvanced ? "Hide" : "Show"} terminal command
          </button>
          {showAdvanced && (
            <>
              <p className="mt-2 text-[11px] text-neutral-500">
                If you're comfortable with the terminal, run this. Otherwise,
                use the guide above.
              </p>
              <code className="mt-2 block rounded bg-neutral-950 px-2 py-1.5 font-mono text-xs text-info-400">
                {advancedCode}
              </code>
            </>
          )}
        </div>
      )}
    </div>
  );
}

// ─── Helpers ────────────────────────────────────────────────────────────────

function detectPlatform(): "mac" | "linux" | "windows" | "other" {
  const p = navigator.platform.toLowerCase();
  if (p.includes("mac")) return "mac";
  if (p.includes("linux")) return "linux";
  if (p.includes("win")) return "windows";
  return "other";
}

function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  return `${m}m ${s % 60}s`;
}

const STEP_ESTIMATES_MS: Record<SubStepId, number> = {
  check: 2_000,
  download: 30_000,
  build: 120_000,
  safety: 20_000,
};

function estimateRemaining(steps: SubStep[]): number | null {
  let remaining = 0;
  for (const step of steps) {
    if (step.status === "succeeded") continue;
    if (step.status === "running" && step.startedAt) {
      const elapsed = Date.now() - step.startedAt;
      remaining += Math.max(0, STEP_ESTIMATES_MS[step.id] - elapsed);
    } else if (step.status === "pending") {
      remaining += STEP_ESTIMATES_MS[step.id];
    }
  }
  return remaining > 0 ? remaining : null;
}

async function prefetchTelegramUrl(
  update: (
    patch: Partial<{
      telegramBotUrl: string | null;
      telegramBotUsername: string | null;
    }>,
  ) => Promise<void>,
): Promise<void> {
  try {
    const envContent = await readConfig("openclaw-vault", ".env").catch(() => "");
    const { telegramToken } = parseEnvKeys(envContent);
    if (!telegramToken) {
      await update({ telegramBotUrl: null, telegramBotUsername: null });
      return;
    }
    const bot = await deriveTelegramBotUrl(telegramToken);
    await update({ telegramBotUrl: bot.url, telegramBotUsername: bot.username });
  } catch {
    // Silently accept failure — Ready falls back to a generic message.
    await update({ telegramBotUrl: null, telegramBotUsername: null });
  }
}
