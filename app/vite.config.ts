import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
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
