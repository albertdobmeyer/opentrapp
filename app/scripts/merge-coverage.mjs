#!/usr/bin/env node
// Merge the vitest UNIT coverage (coverage/coverage-final.json) with the
// Playwright E2E coverage (coverage-e2e/coverage-final.json) into one combined
// number. Both are istanbul-format coverage maps (vitest's v8 provider emits
// istanbul JSON; the E2E side comes from vite-plugin-istanbul via nyc), so they
// merge cleanly by file. Writes coverage-merged/{coverage-summary.json,lcov.info}
// and prints the combined statement/branch/function/line totals.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

import libCoverage from "istanbul-lib-coverage";

const SOURCES = [
  ["unit (vitest)", "coverage/coverage-final.json"],
  ["e2e (playwright)", "coverage-e2e/coverage-final.json"],
];

const map = libCoverage.createCoverageMap({});
for (const [label, file] of SOURCES) {
  if (existsSync(file)) {
    map.merge(JSON.parse(readFileSync(file, "utf8")));
    console.log(`merged: ${label} (${file})`);
  } else {
    console.log(`skipped (missing): ${label} (${file})`);
  }
}

// Accumulate totals across every merged file.
const totals = {
  statements: { total: 0, covered: 0 },
  branches: { total: 0, covered: 0 },
  functions: { total: 0, covered: 0 },
  lines: { total: 0, covered: 0 },
};
for (const file of map.files()) {
  const s = map.fileCoverageFor(file).toSummary();
  for (const k of Object.keys(totals)) {
    totals[k].total += s[k].total;
    totals[k].covered += s[k].covered;
  }
}
const pct = (c, t) => (t === 0 ? 100 : Math.round((c / t) * 10000) / 100);
const summary = {};
for (const k of Object.keys(totals)) {
  summary[k] = { ...totals[k], pct: pct(totals[k].covered, totals[k].total) };
}

mkdirSync("coverage-merged", { recursive: true });
writeFileSync(
  path.join("coverage-merged", "coverage-summary.json"),
  JSON.stringify({ total: summary }, null, 2),
);

console.log("\n── combined coverage (unit + e2e) ─────────────────────────");
for (const k of ["statements", "branches", "functions", "lines"]) {
  const m = summary[k];
  console.log(`  ${k.padEnd(11)} ${String(m.pct).padStart(6)}%  (${m.covered}/${m.total})`);
}
