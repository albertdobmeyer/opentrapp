import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      // `npm run test:coverage` → statement/branch/function/line numbers.
      // Informational (no failing thresholds) so CI never blocks on coverage
      // while we climb toward the CII Silver target; the json-summary feeds the
      // coverage report in coverage.yml.
      // Istanbul (not v8) so unit + E2E coverage share the SAME statement maps
      // and merge cleanly in scripts/merge-coverage.mjs (vite-plugin-istanbul
      // instruments the E2E side with the same instrumenter).
      provider: "istanbul",
      // `json` (coverage-final.json) is what merge-coverage.mjs consumes.
      reporter: ["text-summary", "json-summary", "json", "html"],
      reportsDirectory: "./coverage",
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/**/*.test.{ts,tsx}",
        "src/test-setup.ts",
        "src/**/*.d.ts",
        "src/main.tsx",
        "src/vite-env.d.ts",
      ],
    },
  },
});
