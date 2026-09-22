use serde::Deserialize;

/// A Codex thread — the conversation. Replaces OpenCode's `Session`.
#[derive(Deserialize, Debug, Clone)]
pub struct Thread {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Events the reader forwards to the UI. ServerNotification becomes this
/// after classification, so the UI never sees raw JSON.
#[derive(Clone)]
pub enum ServerEvent {
    /// Carries the client handle to the UI exactly once, after the
    /// handshake succeeded.
    Ready { client: crate::client::CodexClient },
    ThreadStarted { id: String },
    TurnStarted { turn_id: String },
    AgentMessageDelta { delta: String },
    ItemCompleted,
    TurnCompleted,
    Disconnected(String),
    /// Responses (which have an `id`, not a `method`) and unknown
    /// notifications land here; the UI ignores them.
    Ignored,
}
