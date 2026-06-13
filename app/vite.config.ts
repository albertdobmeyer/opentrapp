import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import istanbul from "vite-plugin-istanbul";
import path from "path";

// Instrument the source for coverage ONLY when VITE_COVERAGE=true (the E2E
// coverage run). Normal dev/build is completely unaffected — the plugin is not
// even added. The Playwright run reads window.__coverage__ and merges it with
// the vitest unit coverage (scripts/merge-coverage.mjs).
const withCoverage = process.env.VITE_COVERAGE === "true";

export default defineConfig({
  plugins: [
    react(),
    ...(withCoverage
      ? [
          istanbul({
            include: "src/**/*.{ts,tsx}",
            exclude: ["node_modules", "src/**/*.test.{ts,tsx}", "src/test-setup.ts"],
            extension: [".ts", ".tsx"],
            requireEnv: false,
          }),
        ]
      : []),
  ],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    // Tauri 2 webview floors: WebView2 on Windows is evergreen Chromium;
    // WKWebView on macOS 11+ ships Safari 14+. We let Vite emit modern
    // syntax and rely on the webview to interpret it natively, rather
    // than asking esbuild's post-bundle transform to down-compile (which
    // fails on common destructuring patterns under esbuild 0.28+).
    target: "esnext",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
