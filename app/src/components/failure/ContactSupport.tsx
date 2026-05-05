import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  HeartHandshake,
  Clipboard,
  Mail,
  Github,
  RotateCw,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { useState } from "react";

import { useToast } from "@/lib/ToastContext";

import packageJson from "../../../package.json";

import type { ClassifiedError } from "@/lib/errors";

const APP_VERSION = packageJson.version;

interface ContactSupportProps {
  classified: ClassifiedError;
  /** Optional fallback retry — gives users a last chance if the failure was transient. */
  onRetry?: () => void;
  /** Override for context-specific framing. */
  titleOverride?: string;
}

const SUPPORT_EMAIL = "support@lobster-trapp.com";
const GITHUB_ISSUE_URL =
  "https://github.com/albertdobmeyer/lobster-trapp/issues/new?template=bug.md";

/**
 * Level 3 of the failure cascade per spec 06. Shown after Level 2 retry has failed
 * or for unrecoverable errors. Offers redacted diagnostic bundle + email + GitHub
 * with technical details collapsed by default.
 */
export default function ContactSupport({
  classified,
  onRetry,
  titleOverride,
}: ContactSupportProps) {
  const { addToast } = useToast();
  const [copying, setCopying] = useState(false);
  const [showTechnical, setShowTechnical] = useState(false);

  async function handleCopyDiagnostics() {
    setCopying(true);
    try {
      const bundle = await invoke<string>("generate_diagnostic_bundle");
      await writeText(bundle);
      addToast({
        type: "success",
        title: "Copied!",
        message: "Now paste it in an email or GitHub issue.",
      });
    } catch (error) {
      addToast({
        type: "error",
        title: "Couldn't copy diagnostics",
        message:
          error instanceof Error ? error.message : "Try again in a moment.",
      });
    } finally {
      setCopying(false);
    }
  }

  function openEmail() {
    const subject = encodeURIComponent(
      `Lobster-TrApp needs help [v${APP_VERSION}]`
    );
    const body = encodeURIComponent(
      `[Paste the copied diagnostic info here]\n\nWhat were you trying to do when this happened?\n\n`
    );
    window.open(`mailto:${SUPPORT_EMAIL}?subject=${subject}&body=${body}`);
  }

  function openGithub() {
    window.open(GITHUB_ISSUE_URL, "_blank", "noopener");
  }

  return (
    <div className="mx-auto max-w-lg py-12 px-4 animate-slide-in">
      <div className="text-center">
        <div
          className="mx-auto mb-6 flex h-20 w-20 items-center justify-center rounded-full bg-info-500/10 text-info-400"
          aria-hidden
        >
          <HeartHandshake size={40} strokeWidth={1.5} />
        </div>
        <h2 className="mb-2 text-2xl font-semibold text-neutral-100">
          {titleOverride ?? "Still having trouble"}
        </h2>
        <p className="mb-8 text-sm text-neutral-400">
          We're sorry this isn't working. Here's how to get help quickly:
        </p>
      </div>

      <div className="card-raised mb-6">
        <h3 className="mb-2 text-base font-semibold text-neutral-100">
          📋 Copy diagnostic info
        </h3>
        <p className="mb-4 text-xs text-neutral-400">
          We'll prepare a small text file with everything our team needs to help
          you. No passwords or personal data.
        </p>
        <button
          type="button"
          onClick={handleCopyDiagnostics}
          disabled={copying}
          className="btn btn-md btn-primary w-full"
        >
          <Clipboard size={16} />
          {copying ? "Preparing…" : "Copy to clipboard"}
        </button>
      </div>

      <p className="mb-3 text-center text-xs text-neutral-500">
        Then paste it into one of these:
      </p>
      <div className="mb-8 flex flex-col gap-2">
        <button
          type="button"
          onClick={openEmail}
          className="btn btn-md btn-secondary"
        >
          <Mail size={16} />
          Email support
        </button>
        <button
          type="button"
          onClick={openGithub}
          className="btn btn-md btn-ghost"
        >
          <Github size={16} />
          Post on GitHub
        </button>
      </div>

      {onRetry && (
        <p className="mb-6 text-center text-xs text-neutral-500">
          Or{" "}
          <button
            type="button"
            onClick={onRetry}
            className="inline-flex items-center gap-1 text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
          >
            <RotateCw size={11} />
            try once more
          </button>
        </p>
      )}

      <div className="border-t border-neutral-800 pt-4">
        <button
          type="button"
          onClick={() => { setShowTechnical((v) => !v); }}
          className="flex items-center gap-1 text-[11px] text-neutral-600 hover:text-neutral-400"
        >
          {showTechnical ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
          {showTechnical ? "Hide" : "Show"} technical details
        </button>
        {showTechnical && (
          <div className="mt-3 space-y-2 text-[11px]">
            <div className="text-neutral-500">
              <span className="text-neutral-600">Category:</span>{" "}
              {classified.severity}
            </div>
            <pre className="rounded-md bg-neutral-950 p-3 font-mono text-neutral-400 whitespace-pre-wrap break-all">
              {classified.technicalDetails}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
