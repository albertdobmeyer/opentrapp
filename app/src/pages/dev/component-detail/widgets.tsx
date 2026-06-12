import type { ArgLike } from "./helpers";
import type { CommandResult } from "@/lib/types";
import type { ReactNode } from "react";



const DANGER_PILL: Record<string, string> = {
  destructive: "pill-danger",
  caution: "pill-warning",
};

/** A titled card section. */
export function Panel({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <div>
      <h2 className="mb-2 text-sm font-semibold text-neutral-200">{title}</h2>
      <div className="card-dev">{children}</div>
    </div>
  );
}

/** Danger-level pill, colour-mapped (avoids a nested ternary). */
export function DangerPill({ danger }: { danger: string }) {
  const cls = DANGER_PILL[danger] ?? "pill-neutral";
  return <span className={`pill ${cls}`}>{danger}</span>;
}

/** Render one form field for an arg, dispatched by type (no nested ternary). */
function Field({
  arg,
  value,
  onSet,
}: {
  arg: ArgLike;
  value: string;
  onSet: (v: string) => void;
}) {
  if (arg.type === "boolean") {
    return (
      <select
        className="input mt-1"
        value={value}
        onChange={(e) => {
          onSet(e.target.value);
        }}
      >
        <option value="">—</option>
        <option value="true">true</option>
        <option value="false">false</option>
      </select>
    );
  }
  if (arg.options.length > 0) {
    return (
      <select
        className="input mt-1"
        value={value}
        onChange={(e) => {
          onSet(e.target.value);
        }}
      >
        <option value="">—</option>
        {arg.options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
          </option>
        ))}
      </select>
    );
  }
  return (
    <input
      className="input mt-1"
      type={arg.type === "number" ? "number" : "text"}
      value={value}
      onChange={(e) => {
        onSet(e.target.value);
      }}
    />
  );
}

/** A generic arg-form: one labelled field per arg/input. */
export function ArgFields({
  args,
  values,
  onChange,
}: {
  args: ArgLike[];
  values: Record<string, string>;
  onChange: (next: Record<string, string>) => void;
}) {
  if (args.length === 0) return null;
  return (
    <div className="mt-2 space-y-2">
      {args.map((a) => (
        <label key={a.id} className="block">
          <span className="text-xs text-neutral-400">
            {a.name}
            {a.required ? " *" : ""}
          </span>
          <Field
            arg={a}
            value={values[a.id] ?? ""}
            onSet={(v) => {
              onChange({ ...values, [a.id]: v });
            }}
          />
          {a.description && (
            <span className="mt-0.5 block text-xs text-neutral-600">
              {a.description}
            </span>
          )}
        </label>
      ))}
    </div>
  );
}

/** A command's stdout/stderr/exit result. */
export function ResultPanel({ result }: { result: CommandResult }) {
  const ok = result.exit_code === 0;
  return (
    <div className="mt-2 rounded-md border border-neutral-800 bg-neutral-900 p-2">
      <p className="mb-1 flex items-center gap-2 text-xs">
        <span className={`pill ${ok ? "pill-safe" : "pill-danger"}`}>
          exit {result.exit_code}
        </span>
        <span className="text-neutral-600">{result.duration_ms} ms</span>
      </p>
      {result.stdout && (
        <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words font-mono text-xs text-neutral-300">
          {result.stdout}
        </pre>
      )}
      {result.stderr && (
        <pre className="max-h-32 overflow-auto whitespace-pre-wrap break-words font-mono text-xs text-danger-400">
          {result.stderr}
        </pre>
      )}
    </div>
  );
}
