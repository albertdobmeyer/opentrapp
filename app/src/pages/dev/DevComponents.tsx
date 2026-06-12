import { ChevronRight, Loader2 } from "lucide-react";
import { Link } from "react-router-dom";

import { useManifests } from "@/hooks/useManifests";

/**
 * Discovered-component index (developer mode). Projects whatever components the
 * backend discovered (`list_components`) as a list of links into their generic
 * per-component dashboard — un-parking or adding a component makes a new row
 * appear here with zero changes. This is the entry point that makes
 * DevComponentDetail reachable (the projection vision: the GUI shows only the
 * submodules that are actually present).
 */
export default function DevComponents() {
  const { components, loading, error } = useManifests();

  return (
    <section className="max-w-3xl">
      <h1 className="mb-1 text-xl font-semibold text-neutral-100">Components</h1>
      <p className="mb-4 text-sm text-neutral-400">
        Every discovered component and its operations, projected from its
        manifest.
      </p>

      {loading && (
        <p className="flex items-center gap-2 text-sm text-neutral-400">
          <Loader2 size={15} className="animate-spin" /> Discovering…
        </p>
      )}

      {!loading && error && (
        <div className="card-dev">
          <p className="text-sm text-neutral-200">Discovery failed.</p>
          <p className="mt-1 font-mono text-xs text-neutral-500">{error}</p>
        </div>
      )}

      {!loading && !error && components.length === 0 && (
        <p className="text-sm text-neutral-500">No components discovered.</p>
      )}

      {!loading && !error && components.length > 0 && (
        <div className="space-y-2">
          {components.map(({ manifest }) => {
            const { identity, commands, workflows } = manifest;
            return (
              <Link
                key={identity.id}
                to={`/dev/components/${identity.id}`}
                className="card-interactive flex items-center justify-between gap-3 p-3"
              >
                <div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-neutral-100">
                      {identity.name}
                    </span>
                    <span className="pill pill-neutral">{identity.role}</span>
                    <span className="font-mono text-xs text-neutral-500">
                      v{identity.version}
                    </span>
                  </div>
                  <p className="mt-0.5 text-xs text-neutral-500">
                    {commands.length} command
                    {commands.length === 1 ? "" : "s"} · {workflows.length}{" "}
                    workflow{workflows.length === 1 ? "" : "s"}
                  </p>
                </div>
                <ChevronRight size={16} className="text-neutral-600" />
              </Link>
            );
          })}
        </div>
      )}
    </section>
  );
}
