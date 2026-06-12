import type { Arg } from "@/lib/types";

/**
 * The minimal shape both manifest command args and workflow inputs reduce to,
 * so a single arg-form renderer serves both (the projection is generic — it does
 * not special-case any component).
 */
export type ArgLike = Pick<Arg, "id" | "name" | "type" | "required" | "options"> & {
  description?: string;
  default?: unknown;
};

/** Seed an arg-form value map from declared defaults (primitives only). */
export function initialArgValues(args: ArgLike[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const a of args) {
    const d = a.default;
    out[a.id] =
      typeof d === "string" || typeof d === "number" || typeof d === "boolean"
        ? String(d)
        : "";
  }
  return out;
}

/** Drop empty fields so an unset optional arg is omitted, not sent as "". */
export function nonEmpty(values: Record<string, string>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(values)) {
    if (v !== "") out[k] = v;
  }
  return out;
}
