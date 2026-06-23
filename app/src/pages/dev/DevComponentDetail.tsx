import { AlertTriangle, Loader2, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useParams } from "react-router-dom";


import { CommandRow } from "./component-detail/CommandRow";
import { ConfigRow } from "./component-detail/ConfigRow";
import { HealthRow } from "./component-detail/HealthRow";
import { Panel } from "./component-detail/widgets";
import { WorkflowRow } from "./component-detail/WorkflowRow";

import { getComponent, getStatus } from "@/lib/tauri";
import {
  COMMAND_GROUP_LABELS,
  COMMAND_GROUP_ORDER,
  type DiscoveredComponent,
} from "@/lib/types";

/**
 * Generic, manifest-driven per-component dashboard (developer mode).
 *
 * This view is a PROJECTION (CLAUDE.md §5, the generic-backend constraint): it
 * reads a component's `component.yml` via `get_component` and renders the six
 * manifest sections (identity, status, commands, configs, health, workflows)
 * with no knowledge of what any specific component does. Every interactive
 * affordance routes through the existing generic Tauri channel. Un-parking or
 * adding a component makes a new dashboard appear here with zero changes to this
 * file — that is the projection vision.
 *
 * Developer concepts are permitted here: this is `/dev`, not the user-facing
 * surface the §3 jargon ban governs.
 */
export default function DevComponentDetail() {
  const { id } = useParams<{ id: string }>();
  const [comp, setComp] = useState<DiscoveredComponent | null>(null);
  const [currentState, setCurrentState] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async (componentId: string) => {
    setLoading(true);
    setError(null);
    try {
      setComp(await getComponent(componentId));
      try {
        const st = await getStatus(componentId);
        setCurrentState(st.state_id);
      } catch {
        setCurrentState(null); // status is best-effort; never block the view
      }
    } catch (error_) {
      setError(error_ instanceof Error ? error_.message : String(error_));
      setComp(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (id) void load(id);
  }, [id, load]);

  const reload = () => {
    if (id) void load(id);
  };

  if (loading) {
    return (
      <section className="max-w-3xl">
        <p className="flex items-center gap-2 text-sm text-neutral-400">
          <Loader2 size={15} className="animate-spin" /> Loading {id}…
        </p>
      </section>
    );
  }

  if (error || !comp) {
    return (
      <section className="max-w-3xl">
        <h1 className="mb-1 text-xl font-semibold text-neutral-100">
          {id ?? "Component"}
        </h1>
        <div className="card-dev flex items-start gap-2">
          <AlertTriangle size={16} className="mt-0.5 text-danger-400" />
          <div>
            <p className="text-sm text-neutral-200">
              Could not load this component.
            </p>
            <p className="mt-1 font-mono text-xs text-neutral-500">
              {error ?? "not found"}
            </p>
            <button
              type="button"
              className="btn btn-sm btn-ghost mt-3"
              onClick={reload}
            >
              <RefreshCw size={13} /> Retry
            </button>
          </div>
        </div>
      </section>
    );
  }

  return (
    <ComponentSections
      comp={comp}
      currentState={currentState}
      onReload={reload}
    />
  );
}

function ComponentSections({
  comp,
  currentState,
  onReload,
}: {
  comp: DiscoveredComponent;
  currentState: string | null;
  onReload: () => void;
}) {
  const { identity, status, commands, configs, health, workflows } =
    comp.manifest;

  const stateLabel = status?.states.find((s) => s.id === currentState)?.label;

  const groupedCommands = COMMAND_GROUP_ORDER.map((group) => ({
    group,
    items: commands
      .filter((c) => c.group === group)
      .sort((a, b) => a.sort_order - b.sort_order),
  })).filter((g) => g.items.length > 0);

  return (
    <section className="max-w-3xl space-y-6">
      <header>
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-xl font-semibold text-neutral-100">
            {identity.name}
          </h1>
          <span className="pill pill-neutral">{identity.role}</span>
          <span className="font-mono text-xs text-neutral-500">
            {identity.id} · v{identity.version}
          </span>
        </div>
        {identity.description && (
          <p className="mt-1 text-sm text-neutral-400">{identity.description}</p>
        )}
        <button
          type="button"
          className="btn btn-sm btn-ghost mt-3"
          onClick={onReload}
        >
          <RefreshCw size={13} /> Refresh
        </button>
      </header>

      {status && status.states.length > 0 && (
        <Panel title="Status">
          <div className="flex flex-wrap items-center gap-2">
            {status.states.map((s) => (
              <span
                key={s.id}
                className={`pill ${
                  s.id === currentState ? "pill-info" : "pill-neutral"
                }`}
              >
                {s.label}
                {s.id === currentState ? " · current" : ""}
              </span>
            ))}
          </div>
          {currentState && !stateLabel && (
            <p className="mt-2 font-mono text-xs text-neutral-500">
              current: {currentState}
            </p>
          )}
        </Panel>
      )}

      {commands.length > 0 && (
        <Panel title="Commands">
          <div className="space-y-4">
            {groupedCommands.map(({ group, items }) => (
              <div key={group}>
                <p className="mb-2 text-xs font-medium uppercase tracking-wider text-neutral-500">
                  {COMMAND_GROUP_LABELS[group]}
                </p>
                <div className="space-y-2">
                  {items.map((cmd) => (
                    <CommandRow
                      key={cmd.id}
                      componentId={identity.id}
                      command={cmd}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        </Panel>
      )}

      {configs.length > 0 && (
        <Panel title="Configs">
          <div className="space-y-2">
            {configs.map((cfg) => (
              <ConfigRow key={cfg.path} componentId={identity.id} config={cfg} />
            ))}
          </div>
        </Panel>
      )}

      {health.length > 0 && (
        <Panel title="Health">
          <div className="space-y-2">
            {health.map((probe) => (
              <HealthRow key={probe.id} componentId={identity.id} probe={probe} />
            ))}
          </div>
        </Panel>
      )}

      {workflows.length > 0 && (
        <Panel title="Workflows">
          <div className="space-y-2">
            {workflows.map((wf) => (
              <WorkflowRow key={wf.id} componentId={identity.id} workflow={wf} />
            ))}
          </div>
        </Panel>
      )}
    </section>
  );
}
