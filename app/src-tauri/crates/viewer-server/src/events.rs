//! SCAFFOLD — the `/api/events` WebSocket multiplexer (spec §1.2 / ADR-0022 §4).
//!
//! ONE WS carries all named events, mirroring Tauri's emit/listen. The frontend's `listen()`
//! hooks collapse into a single client that dispatches by `event` name. Message shape:
//!   { "event": "<name>", "payload": { … } }
//! Auth in the FIRST FRAME (not a query-string token — that is the leak we designed out, §2.3).
//! The WS is a fast path only; `markers remain truth` (ADR-0019) — clients re-seed from
//! get_perimeter_state / get_status on connect + reconnect.
//!
//! The 8 events (producer → payload):
//!   perimeter-state-changed   supervisor watchdog ~30s   PerimeterStatus
//!   assistant-status-changed  status aggregator          AssistantStatusSnapshot
//!   bootstrap-step-started    bootstrap                  {step,total_steps,current,detail}
//!   bootstrap-step-failed     bootstrap                  {cause,message}
//!   stream-line               start_stream (per line)    {component_id,command_id,line,stream}  ← bursty; spike must prove this carries
//!   stream-end                stream on exit             {component_id,command_id,exit_code}
//!   sentinel-activity-changed sentinel                   SentinelActivity
//!   telegram-bot-resolved     credentials auto-activate  {url,username}
//!
//! Implementation note: bridge opentrapp-core's internal event bus (the source the Tauri
//! `app.emit()` calls used) to a broadcast channel fanned out to each authed WS client.

// pub fn router() -> axum::Router { Router::new().route("/api/events", get(ws_upgrade)) }
// async fn ws_upgrade(...) { /* verify Origin (§2.2) + first-frame bearer (§2.3), then fan out */ }

#![allow(dead_code)]
