//! Wire envelope wrapping a `Command` (client -> server) or `ServerEvent`
//! (server -> client). Per 02_TECHNICAL_ARCHITECTURE.md ("Wire envelope")
//! and 06_MULTIPLAYER_AND_NETWORK_PROTOCOL.md ("Command envelope").

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::command::Command;
use crate::event::ServerEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Command,
    Event,
}

/// Client -> server envelope. `expected_revision` implements the optimistic
/// concurrency check from 02_TECHNICAL_ARCHITECTURE.md ("Career revision"):
/// the server rejects with `STALE_REVISION` if it doesn't match current
/// state for commands where order matters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub protocol_version: u32,
    pub message_id: Uuid,
    pub kind: MessageKind,
    pub career_id: Uuid,
    pub manager_id: Uuid,
    /// Only meaningful for `kind == Command`; commands that don't care about
    /// ordering (mark message read, cosmetic prefs) may omit it.
    #[serde(default)]
    pub expected_revision: Option<u64>,
    pub payload: EnvelopePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvelopePayload {
    Command(Command),
    Event(ServerEvent),
}

impl Envelope {
    /// Every state-changing command needs a UUID `command_id` for
    /// idempotent retry (02_TECHNICAL_ARCHITECTURE.md, "Idempotency").
    /// Here that's `message_id` — one field serves both purposes since a
    /// command envelope only ever carries one command.
    pub fn command_id(&self) -> Uuid {
        self.message_id
    }
}
