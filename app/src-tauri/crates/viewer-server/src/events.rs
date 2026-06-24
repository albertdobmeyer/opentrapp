//! The `/api/events` WebSocket multiplexer (spec §1.2 / ADR-0022 §4).
//!
//! ONE WS carries all named events, mirroring Tauri's emit/listen. The frontend's `listen()`
//! hooks collapse into a single client dispatching by `event` name. Message shape:
//!   { "event": "<name>", "payload": { … } }
//! Auth is on the HANDSHAKE: the §2 security middleware enforces Host + Origin + the HttpOnly
//! bearer COOKIE (the browser sends it automatically on the upgrade request). First-frame auth was
//! impossible — JS can't read the HttpOnly cookie to send it — so the handshake cookie + the Origin
//! check (anti-CSWSH) is the auth, consistent with every other `/api/*` route. `markers remain
//! truth` (ADR-0019): the WS is a fast path; clients re-seed from get_status on connect + reconnect.
//!
//! SPIKE: proves the bursty `stream-line` path carries (risk item #3). The migration bridges
//! opentrapp-core's event bus (the source the Tauri `app.emit()` calls used) to a broadcast
//! channel fanned out to each authed client.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};

/// The `/api/events` WS route. Auth is on the handshake (the §2 middleware that wraps the whole
/// router enforces Host + Origin + the bearer cookie), so this route adds no state of its own.
pub fn router() -> Router {
    Router::new().route("/api/events", get(ws_upgrade))
}

async fn ws_upgrade(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle)
}

async fn handle(mut socket: WebSocket) {
    // Auth happened on the HANDSHAKE (the §2 middleware enforced Host + Origin + the bearer cookie),
    // so a connected socket is already authenticated — no first-frame check.

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
