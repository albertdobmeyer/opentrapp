//! The `/api/events` WebSocket multiplexer (spec §1.2 / ADR-0022 §4).
//!
//! ONE WS carries all named events, mirroring Tauri's emit/listen. The frontend's `listen()`
//! hooks collapse into a single client dispatching by `event` name. Message shape:
//!   { "event": "<name>", "payload": { … } }
//! Auth in the FIRST FRAME (not a query-string token — that is the leak we designed out, §2.3).
//! Host + Origin are already enforced by the security middleware on the handshake; the bearer is
//! verified here from the first frame. `markers remain truth` (ADR-0019): the WS is a fast path;
//! clients re-seed from get_perimeter_state / get_status on connect + reconnect.
//!
//! SPIKE: proves the bursty `stream-line` path carries (risk item #3). The migration bridges
//! opentrapp-core's event bus (the source the Tauri `app.emit()` calls used) to a broadcast
//! channel fanned out to each authed client.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};

use crate::security::SecurityState;

pub fn router(sec: SecurityState) -> Router {
    Router::new().route("/api/events", get(ws_upgrade)).with_state(sec)
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(sec): State<SecurityState>) -> Response {
    ws.on_upgrade(move |socket| handle(socket, sec))
}

async fn handle(mut socket: WebSocket, sec: SecurityState) {
    // §2.3 — FIRST-FRAME bearer auth. The very first text frame must be the bearer; anything
    // else closes the socket. (Host + Origin were already checked on the handshake.)
    let authed = matches!(
        socket.recv().await,
        Some(Ok(Message::Text(ref t))) if crate::security::bearer_ok(Some(t.as_str()), sec.token.as_str())
    );
    if !authed {
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    // SPIKE: prove the bursty stream-line path carries — emit a stream-line then stream-end,
    // exactly the shape `start_stream` will fan out per line (spec §1.2).
    let line = serde_json::json!({
        "event": "stream-line",
        "payload": { "component_id": "agent", "command_id": "demo", "line": "hello from the perimeter", "stream": "stdout" }
    });
    let _ = socket.send(Message::Text(line.to_string().into())).await;
    let end = serde_json::json!({
        "event": "stream-end",
        "payload": { "component_id": "agent", "command_id": "demo", "exit_code": 0 }
    });
    let _ = socket.send(Message::Text(end.to_string().into())).await;
    let _ = socket.send(Message::Close(None)).await;
}
