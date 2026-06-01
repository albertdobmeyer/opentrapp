import { open as openUrl } from "@tauri-apps/plugin-shell";
import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  KeyRound,
  Network,
  ScanSearch,
  Shield,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";

import SentinelActivityBadge from "@/components/user/SentinelActivityBadge";
import { useHero } from "@/hooks/useHero";

interface Layer {
  icon: LucideIcon;
  title: string;
  body: string;
}

/**
 * The five protective layers, in plain language. The order matches the
 * defense-in-depth flow (assistant runtime → skill scan → key handling →
 * allowlist → kernel-level egress), so reading top-to-bottom is also reading
 * the journey of a request.
 */
const LAYERS: Layer[] = [
  {
    icon: Shield,
    title: "Assistant runs walled off",
    body:
      "Your assistant runs inside a locked-down container with no access to the host filesystem, no internet by default, and a minimal set of system calls. If it's tricked into running malicious code, the damage is contained to that container.",
  },
  {
    icon: ScanSearch,
    title: "Every skill is scanned",
    body:
      "Before any skill from a third-party registry is installed, it's downloaded into a separate container and checked against 87 known-bad patterns. Suspect skills are rebuilt from scratch — the original file is discarded.",
  },
  {
    icon: KeyRound,
    title: "Your API key is never seen by the assistant",
    body:
      "The assistant sends a placeholder. A secure gateway substitutes your real Anthropic key only at the moment of sending the request to Anthropic, then strips it from any visible log. A compromised assistant can't leak the key because it never sees it.",
  },
  {
    icon: Network,
    title: "Only allowlisted destinations are reachable",
    body:
      "The gateway rejects any request that isn't on a short list of trusted hosts (Anthropic, Telegram, the skill registry). DNS rebinding tricks are caught by a second check after the hostname is resolved.",
  },
  {
    icon: ShieldCheck,
    title: "Internal addresses are blocked at the kernel level",
    body:
      "A separate egress container drops, at the operating-system level, any outgoing packet aimed at internal addresses (cloud metadata, local network, loopback). Even if every other layer failed, this layer would still hold.",
  },
];

export default function SecurityMonitor() {
  const { snapshot, loading } = useHero();
  const alerts = snapshot.alerts;
  const showAlerts = !loading && alerts.length > 0;

  async function open(href: string) {
    try {
      await openUrl(href);
    } catch {
      window.open(href, "_blank", "noopener,noreferrer");
    }
  }

  return (
    <div className="mx-auto max-w-3xl px-4 py-10 animate-fade-in">
      <header className="mb-8">
        <div className="mb-3 flex items-start justify-between gap-4">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-primary-500/10 text-primary-400">
            <ShieldCheck size={24} strokeWidth={1.75} />
          </div>
          <SentinelActivityBadge />
        </div>
        <h1 className="text-2xl font-semibold text-neutral-100">Security</h1>
        <p className="mt-2 text-sm text-neutral-400">
          Five independent layers protect your assistant. Each layer catches a
          different class of mistake; one failing does not produce a problem.
        </p>
        <p className="mt-2 text-sm text-neutral-500">
          A small on-device check also watches for unusual activity in the
          background. The badge above shows what it's doing — it only works
          harder when something genuinely needs a closer look.
        </p>
      </header>

      {showAlerts && (
        <section className="mb-6 rounded-xl border border-amber-500/30 bg-amber-500/5 p-5">
          <div className="mb-3 flex items-center gap-2 text-amber-400">
            <AlertTriangle size={18} strokeWidth={1.75} />
            <h2 className="text-sm font-semibold">
              {alerts.length === 1
                ? "1 thing needs your attention"
                : `${alerts.length} things need your attention`}
            </h2>
          </div>
          <ul className="space-y-3">
            {alerts.map((a) => (
              <li key={a.id} className="text-sm">
                <div className="font-medium text-neutral-100">{a.title}</div>
                {a.body && (
                  <div className="mt-0.5 text-neutral-400">{a.body}</div>
                )}
                {a.cta_label && a.cta_to && (
                  <a
                    href={a.cta_to}
                    className="mt-1 inline-block text-sm font-medium text-primary-400 hover:text-primary-300"
                  >
                    {a.cta_label}
                  </a>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}

      {!showAlerts && !loading && (
        <section className="mb-6 flex items-center gap-3 rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-4 text-sm text-emerald-300">
          <CheckCircle2 size={18} strokeWidth={1.75} />
          <span>All five layers are running and healthy.</span>
        </section>
      )}

      <div className="grid gap-3">
        {LAYERS.map((layer, idx) => (
          <article
            key={layer.title}
            className="rounded-xl border border-neutral-800 bg-neutral-900/60 p-5"
          >
            <div className="flex items-start gap-4">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-neutral-800 text-primary-400">
                <layer.icon size={20} strokeWidth={1.75} />
              </div>
              <div className="flex-1">
                <div className="mb-1 flex items-baseline gap-2">
                  <span className="text-xs font-medium uppercase tracking-wider text-neutral-500">
                    Layer {idx + 1}
                  </span>
                </div>
                <h2 className="mb-1 text-base font-semibold text-neutral-100">
                  {layer.title}
                </h2>
                <p className="text-sm text-neutral-400">{layer.body}</p>
              </div>
            </div>
          </article>
        ))}
      </div>

      <footer className="mt-8 rounded-xl border border-neutral-800 bg-neutral-900/40 p-5">
        <h2 className="mb-2 text-sm font-semibold text-neutral-200">
          For the technically curious
        </h2>
        <p className="mb-3 text-sm text-neutral-400">
          The full architecture, threat model, and what each layer actually
          catches are published openly — you can audit, fork, and verify every
          line.
        </p>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() =>
              void open(
                "https://github.com/albertdobmeyer/opentrapp/blob/main/docs/perimeter-explained.md",
              )
            }
            className="btn btn-sm btn-secondary"
          >
            <ExternalLink size={14} />
            One-page architecture
          </button>
          <button
            type="button"
            onClick={() =>
              void open(
                "https://github.com/albertdobmeyer/opentrapp/blob/main/docs/threat-model.md",
              )
            }
            className="btn btn-sm btn-secondary"
          >
            <ExternalLink size={14} />
            Full threat model
          </button>
        </div>
      </footer>
    </div>
  );
}
