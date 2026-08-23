//! Server -> client messages. Per 02_TECHNICAL_ARCHITECTURE.md
//! ("Server response model") and 06_MULTIPLAYER_AND_NETWORK_PROTOCOL.md
//! ("WebSocket events", "Broadcast audiences").

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ProtocolError;
use crate::versions::VersionSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum ServerEvent {
    Hello(HelloBody),
    CommandAck(CommandAckBody),
    CommandRejected(CommandRejectedBody),
    StateDelta(StateDeltaBody),
    ViewSnapshot(ViewSnapshotBody),
    GameTimeChanged(GameTimeChangedBody),
    ReadyStateChanged(ReadyStateChangedBody),
    MatchOpened(MatchOpenedBody),
    MatchEventBatch(MatchEventBatchBody),
    MatchState(MatchStateBody),
    MatchFinished(MatchFinishedBody),
    ServerNotice(ServerNoticeBody),
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloBody {
    pub career_id: Uuid,
    pub server_versions: VersionSet,
    pub current_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAckBody {
    pub command_id: Uuid,
    pub applied_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRejectedBody {
    pub command_id: Uuid,
    pub current_revision: u64,
    pub error: ProtocolError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDeltaBody {
    pub from_revision: u64,
    pub to_revision: u64,
    /// Opaque JSON patch payload; kept generic here so domain/view types
    /// (owned by higher crates) don't become a dependency of the protocol
    /// crate itself.
    pub changes: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSnapshotBody {
    pub revision: u64,
    pub view_name: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTimeChangedBody {
    pub career_year: i32,
    pub career_month: u8,
    pub career_day: u8,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyStateChangedBody {
    pub manager_id: Uuid,
    pub is_ready: bool,
    pub other_manager_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchOpenedBody {
    pub match_id: Uuid,
    pub fixture_id: Uuid,
    pub home_club_id: Uuid,
    pub away_club_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEventBatchBody {
    pub match_id: Uuid,
    pub from_seq: u64,
    pub to_seq: u64,
    pub events: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchStateBody {
    pub match_id: Uuid,
    pub phase: String,
    pub match_second: u32,
    pub home_score: u8,
    pub away_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchFinishedBody {
    pub match_id: Uuid,
    pub home_score: u8,
    pub away_score: u8,
    pub report: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerNoticeSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerNoticeBody {
    pub severity: ServerNoticeSeverity,
    /// A stable key for client-side localization, not pre-rendered text
    /// (mirrors ProtocolError's code+params approach).
    pub message_key: String,
}
