//! SCAFFOLD — viewer-server spike entry point (ADR-0022 §0 / spec §0).
//!
//! Not working code. Shows the wiring the spike must implement. See README.md.
//!
//! Flow: bind 127.0.0.1:0 → build router (security layer + routes + /api/events WS +
//! /api/session + SPA static) → write {port,token} to a 0600 file → open the browser to a
//! bare loopback URL carrying a single-use launch nonce in the URL *fragment* → on session
//! end / idle, tear down. ON-DEMAND ONLY: never start this from the always-on daemon path.

mod security;
mod routes;
mod events;
mod session;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Bind loopback + ephemeral port. ASSERT the bound addr is loopback (spec §2.1).
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    assert!(addr.ip().is_loopback(), "viewer-server MUST bind loopback only");

    // 2. Mint a ≥256-bit token + a single-use launch nonce; persist {port,token} 0600 (spec §2.3).
    let _session = session::Session::new(addr.port()); // TODO: writes ~/.opentrapp/viewer/<…> mode 0600

    // 3. Build the app: routes + WS + session-exchange + SPA fallback, wrapped in the security layer.
    //    let core = opentrapp_core::Engine::open()?;   // shared orchestration handle (no duplication)
    //    let app = routes::router(core)
    //        .merge(events::router())
    //        .merge(session::router())
    //        .fallback(/* serve app/dist/ index.html for SPA routes */)
    //        .layer(security::layer(&_session));        // Host/Origin/token/CSP/limits — spec §2

    // 4. Open the browser to the nonce URL (fragment, not query): http://127.0.0.1:<port>/#n=<nonce>
    //    Use xdg-open/open/start directly (NOT tauri-plugin-shell). Then serve until idle/session-end.
    //    axum::serve(listener, app).await?;

    let _ = (&listener, addr);
    todo!("spike: implement per docs/specs/2026-06-17-loopback-web-gui-implementation.md §0");
}
