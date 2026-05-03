import { useEffect, useRef } from "react";
import { X } from "lucide-react";

export interface HowToStep {
  heading: string;
  body: string;
  /** Optional URL to open in a new tab when the user clicks through. */
  href?: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  title: string;
  steps: HowToStep[];
  /** Label for the "Done, let me enter it" confirmation button. */
  ctaLabel?: string;
}

/**
 * Reusable modal for "Show me how to get one" flows on the Connect step.
 * Text-only numbered walkthroughs at v0.3.0; screenshots are queued for a
 * post-launch revision. Also reused by the Preferences key-change flow,
 * so keep it generic.
 */
export default function HowToModal({ open, onClose, title, steps, ctaLabel }: Props) {
  const closeBtnRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!open) return;
    // Focus the close button on open for keyboard accessibility.
    closeBtnRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="howto-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 animate-fade-in"
      onClick={onClose}
    >
      <div
        className="card-hero relative w-full max-w-xl max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <button
          ref={closeBtnRef}
          type="button"
          onClick={onClose}
          aria-label="Close"
          className="absolute right-4 top-4 rounded-md p-2 text-neutral-400 hover:bg-neutral-800 hover:text-neutral-100"
        >
          <X size={18} />
        </button>

        <h2 id="howto-title" className="mb-6 text-xl font-semibold text-neutral-100">
          {title}
        </h2>

        <ol className="space-y-5">
          {steps.map((step, idx) => (
            <li key={idx} className="flex gap-4">
              <div
                aria-hidden
                className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-primary-500/15 text-sm font-semibold text-primary-400"
              >
                {idx + 1}
              </div>
              <div className="flex-1">
                <h3 className="mb-1 text-sm font-medium text-neutral-100">
                  {step.heading}
                </h3>
                <p className="text-sm leading-relaxed text-neutral-400">
                  {step.body}
                </p>
                {step.href && (
                  <a
                    href={step.href}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="mt-2 inline-block text-xs text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
                  >
                    Open {new URL(step.href).hostname}
                  </a>
                )}
                {/* TODO E.4: inline screenshot goes here */}
              </div>
            </li>
          ))}
        </ol>

        <div className="mt-8 flex justify-end">
          <button type="button" onClick={onClose} className="btn btn-md btn-primary">
            {ctaLabel ?? "Done, let me enter it"}
          </button>
        </div>
      </div>
    </div>
  );
}
