import { ChevronDown, ChevronRight, ExternalLink } from "lucide-react";
import { useState } from "react";

import { detectPlatform } from "./utils";

import type { Platform } from "./utils";

interface MissingRuntimeCardProps {
  onBack: () => void;
  onRetry: () => void;
}

interface InstallRecipe {
  title: string;
  body: string;
  /** Optional terminal command — hidden by default behind a disclosure. */
  advancedCode?: string;
  href: string;
}

const RECIPES: Record<Platform, InstallRecipe> = {
  linux: {
    title: "Linux (Ubuntu/Debian)",
    body: "Open the guide for a one-click installer.",
    advancedCode: "sudo apt install podman podman-compose",
    href: "https://podman.io/docs/installation#installing-on-linux",
  },
  mac: {
    title: "macOS",
    body: "Download the sandbox engine and run the installer.",
    href: "https://podman-desktop.io/downloads",
  },
  windows: {
    title: "Windows",
    body: "Download the sandbox engine and run the installer.",
    href: "https://podman-desktop.io/downloads",
  },
  other: {
    title: "Install a sandbox runner",
    body: "Choose the option for your computer:",
    href: "https://podman-desktop.io/downloads",
  },
};

export function MissingRuntimeCard({ onBack, onRetry }: MissingRuntimeCardProps) {
  const recipe = RECIPES[detectPlatform()];
  return (
    <div className="mx-auto max-w-xl px-4 py-10 animate-fade-in">
      <div className="card-raised">
        <h2 className="mb-2 text-xl font-semibold text-neutral-100">
          One thing missing
        </h2>
        <p className="mb-4 text-sm text-neutral-400">
          You’ll need a sandbox runner installed first. It’s free, takes a
          minute, and is what keeps your assistant safely separated from the
          rest of your computer.
        </p>

        <div className="mb-5 space-y-2">
          <InstallLine {...recipe} />
        </div>

        <p className="mb-6 text-xs text-neutral-500">
          After installing, click “Check again” and we’ll pick it up
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

function InstallLine({ title, body, advancedCode, href }: InstallRecipe) {
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
                If you’re comfortable with the terminal, run this. Otherwise,
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
