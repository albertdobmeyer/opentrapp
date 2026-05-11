import { open as openUrl } from "@tauri-apps/plugin-shell";
import { MessageCircle, Play, RotateCcw, StopCircle } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { useSettings } from "@/hooks/useSettings";
import { useToast } from "@/hooks/useToast";
import { classifyError } from "@/lib/errors";
import { pausePerimeter, resumePerimeter, retryBootstrap, type BootstrapFailureSummary } from "@/lib/tauri";

import type { HeroState } from "@/hooks/useHero";

interface Props {
  state: HeroState;
  loading: boolean;
  onLaunch?: () => void;
  bootstrapFailure?: BootstrapFailureSummary | null;
}

interface Copy {
  title: string;
  subline: string;
  /** Tailwind classes for the illustration ring color. */
  ringTint: string;
  /** Inner-dot tailwind class. */
  dotTint: string;
}

const COPY: Record<HeroState, Copy> = {
  installing: {
    title: "Getting your assistant ready…",
    subline: "We're setting things up in the background.",
    ringTint: "border-primary-500/40 border-primary-500/60",
    dotTint: "bg-primary-500",
  },
  bootstrapping: {
    title: "Setting up your safe room…",
    subline: "First-time setup, ~5 minutes. You can keep working.",
    ringTint: "border-warning-500/40 border-warning-500/60",
    dotTint: "bg-warning-500",
  },
  shell_ready_absent: {
    title: "Ready to launch your assistant",
    subline: "Two quick steps and you're chatting on Telegram.",
    ringTint: "border-primary-500/40 border-primary-500/60",
    dotTint: "bg-primary-500",
  },
  shell_failed: {
    title: "Background setup needs your help",
    subline: "Something stopped the setup. We can try to fix it.",
    ringTint: "border-danger-500/40 border-danger-500/60",
    dotTint: "bg-danger-500",
  },
  running_safely: {
    title: "Your assistant is running safely",
    subline: "Open Telegram to start chatting.",
    ringTint: "border-success-500/40 border-success-500/60",
    dotTint: "bg-success-500",
  },
  starting: {
    title: "Your assistant is starting…",
    subline: "Hang tight — this usually takes a few seconds.",
    ringTint: "border-warning-500/40 border-warning-500/60",
    dotTint: "bg-warning-500",
  },
  recovering: {
    title: "Your assistant is taking a moment",
    subline: "Back in a few seconds.",
    ringTint: "border-warning-500/40 border-warning-500/60",
    dotTint: "bg-warning-500",
  },
  error_perimeter: {
    title: "Your assistant didn't recover",
    subline: "Something stopped it from running. We can try to fix it.",
    ringTint: "border-danger-500/40 border-danger-500/60",
    dotTint: "bg-danger-500",
  },
  error_key: {
    title: "Your Anthropic key isn't working",
    subline: "Update it in Preferences and your assistant will be back.",
    ringTint: "border-danger-500/40 border-danger-500/60",
    dotTint: "bg-danger-500",
  },
  not_setup: {
    title: "Your assistant isn't set up yet",
    subline: "Let's get you started.",
    ringTint: "border-primary-500/40 border-primary-500/60",
    dotTint: "bg-primary-500",
  },
  paused_by_user: {
    title: "Your assistant is stopped",
    subline: "Tap Resume when you're ready.",
    ringTint: "border-neutral-500/40 border-neutral-500/60",
    dotTint: "bg-neutral-500",
  },
};

// ─── Recovery copy taxonomy ───────────────────────────────────────────────

interface RecoveryCopy {
  title: string;
  body: string;
  primaryLabel: string;
  secondaryLabel?: string;
  secondaryHref?: string;
}

const RECOVERY_COPY: Record<string, RecoveryCopy> = {
  "no-container-runtime": {
    title: "Couldn't find a container runtime",
    body: "We need Podman or Docker to keep your assistant in a sealed room. Make sure it's installed and running.",
    primaryLabel: "Try again",
  },
  "image-build-failed": {
    title: "Couldn't build the safe-room components",
    body: "Something stopped us from preparing the safe room on your computer.",
    primaryLabel: "Try again",
    secondaryLabel: "Show details",
  },
  "image-pull-failed": {
    title: "Couldn't download a safe-room component",
    body: "Network hiccup while downloading. Check your connection and try again.",
    primaryLabel: "Try again now",
  },
  "env-write-failed": {
    title: "Couldn't write the configuration file",
    body: "We couldn't create the configuration file. Check disk space and permissions.",
    primaryLabel: "Try again",
  },
  "shell-up-failed": {
    title: "Couldn't start the safe room",
    body: "The safe room couldn't start up. We can try again.",
    primaryLabel: "Try again",
    secondaryLabel: "Get help",
  },
  "shell-verify-failed": {
    title: "Couldn't verify the safe room",
    body: "The safe room started but isn't responding correctly.",
    primaryLabel: "Try again",
    secondaryLabel: "Get help",
  },
};

const DEFAULT_RECOVERY: RecoveryCopy = {
  title: "Background setup needs your help",
  body: "Something stopped the setup. We can try to fix it.",
  primaryLabel: "Try again",
};

// eslint-disable-next-line complexity, max-lines-per-function
export default function HeroStatusCard({ state, loading, onLaunch, bootstrapFailure }: Props) {
  const navigate = useNavigate();
  const { settings } = useSettings();
  const { addToast, removeToast } = useToast();
  const copy = COPY[state];
  const [pauseLoading, setPauseLoading] = useState(false);
  const [showStopConfirm, setShowStopConfirm] = useState(false);
  const [retryLoading, setRetryLoading] = useState(false);
  const [showFailureDetails, setShowFailureDetails] = useState(false);

  const telegramLink =
    settings.telegramBotUrl ??
    (settings.telegramBotUsername
      ? `https://t.me/${settings.telegramBotUsername}?text=Hi`
      : "https://telegram.org");

  async function handleOpenTelegram() {
    try {
      await openUrl(telegramLink);
    } catch {
      window.open(telegramLink, "_blank", "noopener,noreferrer");
    }
  }

  const handleStop = () => {
    setShowStopConfirm(false);
    void runPerimeterToggle({
      setLoading: setPauseLoading,
      addToast,
      removeToast,
      action: pausePerimeter,
      pendingTitle: "Stopping your assistant…",
      pendingMessage: undefined,
      successTitle: "Stopped. Your data is safe. Tap Resume any time.",
      successMessage: undefined,
      errorFallbackTitle: "Couldn't stop",
    });
  };

  const handleRetryBootstrap = () =>
    runPerimeterToggle({
      setLoading: setRetryLoading,
      addToast,
      removeToast,
      action: retryBootstrap,
      pendingTitle: "Retrying setup…",
      pendingMessage: undefined,
      successTitle: "Retrying in the background",
      successMessage: "We'll update you when it's done.",
      errorFallbackTitle: "Couldn't restart setup",
    });

  const handleResume = () =>
    runPerimeterToggle({
      setLoading: setPauseLoading,
      addToast,
      removeToast,
      action: resumePerimeter,
      pendingTitle: "Bringing your assistant back…",
      pendingMessage: "This usually takes about 10 seconds.",
      successTitle: "Your assistant is back online",
      successMessage: undefined,
      errorFallbackTitle: "Couldn't resume",
    });

  if (loading) {
    return (
      <div className="card-hero animate-pulse">
        <div className="mx-auto mb-6 h-20 w-20 rounded-full bg-neutral-800" />
        <div className="mx-auto mb-3 h-7 w-2/3 rounded bg-neutral-800" />
        <div className="mx-auto h-4 w-1/2 rounded bg-neutral-800" />
      </div>
    );
  }

  return (
    <div className="card-hero text-center" aria-live="polite">
      <StatusIllustration ringTint={copy.ringTint} dotTint={copy.dotTint} />

      <h1 className="mb-2 text-3xl font-semibold text-neutral-100">
        {copy.title}
      </h1>
      <p className="mx-auto mb-8 max-w-xl text-sm text-neutral-400">
        {copy.subline}
      </p>

      <div className="flex flex-wrap items-center justify-center gap-3">
        {(state === "installing" || state === "bootstrapping") && (
          <span className="text-xs text-neutral-500">
            Working on it — no action needed.
          </span>
        )}

        {state === "shell_ready_absent" && (
          <button
            type="button"
            onClick={() => { onLaunch?.(); }}
            className="btn btn-lg btn-primary"
          >
            Launch your assistant
          </button>
        )}

        {state === "shell_failed" && (() => {
          const recovery = (bootstrapFailure?.cause ? RECOVERY_COPY[bootstrapFailure.cause] : undefined) ?? DEFAULT_RECOVERY;
          return (
            <>
              <button
                type="button"
                onClick={handleRetryBootstrap}
                className="btn btn-lg btn-primary"
                disabled={retryLoading}
              >
                <RotateCcw size={18} />
                {retryLoading ? "Retrying…" : recovery.primaryLabel}
              </button>
              {recovery.secondaryLabel && (
                recovery.secondaryHref ? (
                  <a href={recovery.secondaryHref} target="_blank" rel="noopener noreferrer" className="btn btn-lg btn-ghost">
                    {recovery.secondaryLabel}
                  </a>
                ) : (
                  <button
                    type="button"
                    onClick={() => { setShowFailureDetails((v) => !v); }}
                    className="btn btn-lg btn-ghost"
                  >
                    {showFailureDetails ? "Hide details" : recovery.secondaryLabel}
                  </button>
                )
              )}
              {!recovery.secondaryLabel && (
                <button type="button" onClick={() => { navigate("/help"); }} className="btn btn-lg btn-ghost">
                  Get help
                </button>
              )}
              {showFailureDetails && bootstrapFailure?.last_error && (
                <div className="mt-4 w-full rounded-md bg-neutral-950 p-3 text-left text-xs text-neutral-400 font-mono whitespace-pre-wrap break-all">
                  {bootstrapFailure.last_error}
                </div>
              )}
            </>
          );
        })()}

        {state === "running_safely" && (
          <div className="flex flex-col items-center gap-3">
            <div className="flex flex-wrap items-center justify-center gap-3">
              <button
                type="button"
                onClick={handleOpenTelegram}
                className="btn btn-lg btn-primary"
              >
                <MessageCircle size={18} />
                Open Telegram
              </button>
              {!showStopConfirm && (
                <button
                  type="button"
                  onClick={() => { setShowStopConfirm(true); }}
                  className="btn btn-lg btn-ghost"
                  disabled={pauseLoading}
                >
                  <StopCircle size={18} />
                  {pauseLoading ? "Stopping…" : "Stop your assistant"}
                </button>
              )}
            </div>
            {showStopConfirm && (
              <div className="rounded-lg border border-neutral-700 bg-neutral-800/60 px-4 py-3 text-center max-w-sm">
                <p className="mb-1 text-sm font-medium text-neutral-200">Stop your assistant?</p>
                <p className="mb-3 text-xs text-neutral-500">
                  It&rsquo;ll stop responding on Telegram until you tap Resume. Your conversation history and installed skills stay safe.
                </p>
                <div className="flex justify-center gap-2">
                  <button
                    type="button"
                    onClick={() => { setShowStopConfirm(false); }}
                    className="btn btn-sm btn-ghost"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    onClick={handleStop}
                    className="btn btn-sm btn-danger"
                    disabled={pauseLoading}
                  >
                    {pauseLoading ? "Stopping…" : "Stop now"}
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {(state === "starting" || state === "recovering") && (
          <span className="text-xs text-neutral-500">
            Working on it — no action needed.
          </span>
        )}

        {state === "error_perimeter" && (
          <>
            <button
              type="button"
              onClick={() => { window.location.reload(); }}
              className="btn btn-lg btn-primary"
            >
              <RotateCcw size={18} />
              Try to fix
            </button>
            <button
              type="button"
              onClick={() => { navigate("/help"); }}
              className="btn btn-lg btn-ghost"
            >
              Get help
            </button>
          </>
        )}

        {state === "error_key" && (
          <button
            type="button"
            onClick={() => { navigate("/preferences"); }}
            className="btn btn-lg btn-primary"
          >
            <RotateCcw size={18} />
            Update your key
          </button>
        )}

        {state === "not_setup" && (
          <button
            type="button"
            onClick={() => { navigate("/setup"); }}
            className="btn btn-lg btn-primary"
          >
            Run setup
          </button>
        )}

        {state === "paused_by_user" && (
          <button
            type="button"
            onClick={handleResume}
            className="btn btn-lg btn-primary"
            disabled={pauseLoading}
          >
            <Play size={18} />
            {pauseLoading ? "Resuming…" : "Resume"}
          </button>
        )}
      </div>
    </div>
  );
}

/**
 * State-coloured pulsing rings — stand-in for the bespoke status-{state}.svg
 * illustrations the page spec calls for. Reuses the wizard's animate-pulse-ring
 * keyframes from globals.css.
 */
function StatusIllustration({ ringTint, dotTint }: { ringTint: string; dotTint: string }) {
  // Split combined tint string into the two border tones.
  const [outer, inner] = ringTint.split(" ");
  return (
    <div
      className="relative mx-auto mb-6 h-20 w-20"
      role="img"
      aria-hidden="true"
    >
      <span
        className={`animate-pulse-ring absolute inset-0 rounded-full border-2 ${outer}`}
      />
      <span
        className={`animate-pulse-ring absolute inset-3 rounded-full border-2 ${inner}`}
        style={{ animationDelay: "0.3s" }}
      />
      <span className={`absolute inset-7 rounded-full ${dotTint}`} />
    </div>
  );
}

interface PerimeterToggleArgs {
  setLoading: (v: boolean) => void;
  addToast: ReturnType<typeof useToast>["addToast"];
  removeToast: ReturnType<typeof useToast>["removeToast"];
  action: () => Promise<void>;
  pendingTitle: string;
  pendingMessage: string | undefined;
  successTitle: string;
  successMessage: string | undefined;
  errorFallbackTitle: string;
}

/**
 * Drives one perimeter pause/resume action through its full toast lifecycle:
 * sticky in-flight → success or classified error. Hoisted out of the
 * component so the host stays focused on render concerns.
 */
async function runPerimeterToggle(args: PerimeterToggleArgs): Promise<void> {
  const {
    setLoading, addToast, removeToast, action,
    pendingTitle, pendingMessage,
    successTitle, successMessage,
    errorFallbackTitle,
  } = args;
  setLoading(true);
  const stickyId = addToast({
    type: "info",
    title: pendingTitle,
    message: pendingMessage,
    duration: 0,
  });
  try {
    await action();
    removeToast(stickyId);
    addToast({ type: "success", title: successTitle, message: successMessage });
  } catch (error) {
    removeToast(stickyId);
    const c = classifyError(error);
    addToast({
      type: "error",
      title: c.title === "Something went wrong" ? errorFallbackTitle : c.title,
      message: c.userMessage,
    });
  } finally {
    setLoading(false);
  }
}
