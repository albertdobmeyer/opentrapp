//! A transport-neutral event bus (ADR-0022 §4). Background work (today: command streaming) emits
//! named events here; transports fan them out — the loopback viewer-server forwards each to its
//! `/api/events` WebSocket clients, and the Tauri layer can bridge them to `app.emit()`. This is the
//! `opentrapp-core` source the Tauri `AppHandle::emit` calls become, so one emission feeds every
//! projection.
//!
//! It is a `tokio::sync::broadcast` channel: many subscribers, each gets every event sent AFTER it
//! subscribed (so subscribe before triggering the work). A send with no live subscribers is a no-op,
//! never an error — an unobserved event is fine (markers remain truth, ADR-0019).

use serde_json::Value;
use tokio::sync::broadcast;

/// One named event + its JSON payload — the shape the WS frame (`{event, payload}`) and the Tauri
/// `emit(event, payload)` both project from.
#[derive(Clone, Debug)]
pub struct EventEnvelope {
    pub event: String,
    pub payload: Value,
}

/// A cloneable handle to the bus. Clones share one underlying channel, so a handle stored in a
/// viewer-server `AppState` and a clone held by a streaming task emit to the same subscribers.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<EventEnvelope>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// A bus with a bounded backlog. A slow subscriber that falls more than `capacity` events behind
    /// gets a `Lagged` signal rather than stalling the sender (the WS handler treats that as "you
    /// missed some — re-seed from the `get_*` reads", ADR-0019).
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(512);
        Self { tx }
    }

    /// Emit a named event. Returns nothing: a send that reaches no subscriber is a no-op by design.
    pub fn emit(&self, event: impl Into<String>, payload: Value) {
        let _ = self.tx.send(EventEnvelope { event: event.into(), payload });
    }

    /// Subscribe to events sent from now on. Each receiver gets its own copy of every future event.
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn a_subscriber_receives_an_emitted_event() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe(); // subscribe BEFORE emitting
        bus.emit("stream-line", json!({ "line": "hello", "stream": "stdout" }));

        let env = rx.recv().await.expect("event delivered");
        assert_eq!(env.event, "stream-line");
        assert_eq!(env.payload["line"], "hello");
    }

    #[tokio::test]
    async fn every_live_subscriber_gets_the_event_and_a_pre_subscribe_emit_is_missed() {
        let bus = EventBus::new();
        bus.emit("pre", json!({})); // no subscribers yet → dropped (no-op), never an error
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        bus.emit("after", json!({ "n": 1 }));

        assert_eq!(a.recv().await.unwrap().event, "after");
        assert_eq!(b.recv().await.unwrap().event, "after");
    }
}
