import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import App from "./App";
import { bootstrapSession } from "./lib/session-bootstrap";
import "./styles/globals.css";

const rootElement = document.querySelector("#root");
if (!rootElement) {
  throw new Error("Root element #root not found in document");
}

// De-Tauri loopback viewer (ADR-0022 §2.3): exchange the launch nonce for the HttpOnly bearer
// cookie BEFORE mounting, so the first `/api/*` calls in the app's effects carry it. A no-op (resolves
// instantly) in the Tauri webview, so the shipped desktop app's startup is unchanged. `.finally` so a
// failed exchange still renders the app (it degrades through normal error handling rather than white-
// screening).
void bootstrapSession().finally(() => {
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </React.StrictMode>,
  );
});
