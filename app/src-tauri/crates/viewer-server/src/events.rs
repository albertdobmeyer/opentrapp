//! The `/api/events` WebSocket multiplexer (spec §1.2 / ADR-0022 §4).
//!
//! ONE WS carries all named events, mirroring Tauri's emit/listen. The frontend's `listen()`
//! hooks collapse into a single client dispatching by `event` name. Message shape:
//!   { "event": "<name>", "payload": { … } }
//! Auth is on the HANDSHAKE: the §2 security middleware enforces Host + Origin + the HttpOnly
//! bearer COOKIE (the browser sends it automatically on the upgrade request). First-frame auth was
//! impossible — JS can't read the HttpOnly cookie to send it — so the handshake cookie + the Origin
//! check (anti-CSWSH) is the auth, consistent with every other `/api/*` route.
//!
//! Each connected client subscribes to the shared `opentrapp-core` [`EventBus`] and forwards every
//! event it emits (today: command streaming's `stream-line` / `stream-end`). `markers remain truth`
//! (ADR-0019): the WS is a fast PUSH path; on a `Lagged` drop the client re-seeds from the `get_*`
//! reads, so it degrades to the polled value, never to a lie.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use opentrapp_core::events::EventBus;
use tokio::sync::broadcast::error::RecvError;

/// The `/api/events` WS route. The bus is the route's state; each client subscribes to it on
/// connect. Auth is on the handshake (the §2 middleware wrapping the whole router).
pub fn router(bus: EventBus) -> Router {
    Router::new().route("/api/events", get(ws_upgrade)).with_state(bus)
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(bus): State<EventBus>) -> Response {
    ws.on_upgrade(move |socket| handle(socket, bus))
}

async fn handle(mut socket: WebSocket, bus: EventBus) {
    // Auth happened on the HANDSHAKE (the §2 middleware enforced Host + Origin + the bearer cookie),
    // so a connected socket is already authenticated. Forward bus events until the client leaves.
    let mut rx = bus.subscribe();
    loop {
        tokio::select! {
            received = rx.recv() => match received {
                Ok(env) => {
                    let frame = serde_json::json!({ "event": env.event, "payload": env.payload });
                    if socket.send(Message::Text(frame.to_string().into())).await.is_err() {
                        break; // client gone
                    }
                }
                // Fell behind the bounded backlog — drop + keep going; the client re-seeds via get_*.
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            },
            // Notice a client close / error. We don't expect app data from the client on this socket.
            incoming = socket.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                _ => {}
            },
        }
    }
}
