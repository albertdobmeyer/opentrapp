import { AlertTriangle, X } from "lucide-react";
import { useNavigate } from "react-router-dom";

import { useAlerts } from "@/hooks/useAlerts";

/**
 * Stack of alerts that ride above the hero on Home. Renders nothing when
 * there are no active (non-dismissed) alerts — it's invisible 90% of the
 * time, which honours P12 ("the app speaks only when it's helping").
 */
export default function ProactiveAlertsBanner() {
  const { alerts, dismiss } = useAlerts();
  const navigate = useNavigate();

  if (alerts.length === 0) return null;

  return (
    <div className="mb-4 space-y-2" role="region" aria-label="Active alerts">
      {alerts.map((alert) => {
        const tone =
          alert.severity === "danger"
            ? "border-danger-500/60 bg-danger-500/10"
            : (alert.severity === "warning"
              ? "border-warning-500/60 bg-warning-500/10"
              : "border-info-500/60 bg-info-500/10");
        const iconTint =
          alert.severity === "danger"
            ? "text-danger-400"
            : (alert.severity === "warning"
              ? "text-warning-400"
              : "text-info-400");
        return (
          <div
            key={alert.id}
            className={`flex items-start gap-3 rounded-lg border px-4 py-3 ${tone}`}
            role="alert"
          >
            <AlertTriangle size={18} className={`mt-0.5 shrink-0 ${iconTint}`} />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium text-neutral-100">{alert.title}</p>
              {alert.body && (
                <p className="mt-1 text-xs text-neutral-300">{alert.body}</p>
              )}
              {alert.cta && (() => {
                const cta = alert.cta;
                return (
                  <button
                    type="button"
                    onClick={() => { navigate(cta.to); }}
                    className="mt-2 text-xs font-medium text-primary-400 underline-offset-4 hover:text-primary-300 hover:underline"
                  >
                    {cta.label}
                  </button>
                );
              })()}
            </div>
            {alert.dismissable && (
              <button
                type="button"
                onClick={() => { dismiss(alert.id); }}
                aria-label={`Dismiss "${alert.title}"`}
                className="rounded p-1 text-neutral-500 transition-colors hover:text-neutral-200"
              >
                <X size={14} />
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
