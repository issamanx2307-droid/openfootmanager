//! Typed commands a manager client can send. Per 02_TECHNICAL_ARCHITECTURE.md
//! ("Command/query split"): clients never get direct SQL, only these.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum Command {
    SetStartingXi(SetStartingXiBody),
    SetTactics(SetTacticsBody),
    SubmitTransferBid(SubmitTransferBidBody),
    RespondTransferOffer(RespondTransferOfferBody),
    SubmitContractOffer(SubmitContractOfferBody),
    SetTrainingPlan(SetTrainingPlanBody),
    MarkReady(MarkReadyBody),
    ApplyLiveMatchCommand(ApplyLiveMatchCommandBody),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStartingXiBody {
    /// Stable domain ids are not necessarily UUIDs: imported FPL records use
    /// identifiers such as `fpl-<source id>`.
    pub fixture_id: String,
    pub player_ids: Vec<String>,
    pub formation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTacticsBody {
    pub formation: String,
    pub mentality: String,
}

/// Money is always an integer count of minor units (e.g. pence), never a
/// float, per 03_DATA_SNAPSHOT_AND_DATABASE.md ("All money uses integer
/// minor units or exact decimal, never float.").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTransferBidBody {
    pub player_id: String,
    pub upfront_minor: i64,
    #[serde(default)]
    pub installments_minor: Vec<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferOfferResponse {
    Accept,
    Reject,
    Counter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespondTransferOfferBody {
    pub offer_id: String,
    pub response: TransferOfferResponse,
    #[serde(default)]
    pub counter_upfront_minor: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitContractOfferBody {
    pub player_id: String,
    pub weekly_wage_minor: i64,
    pub contract_end_year: u16,
    pub contract_end_month: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTrainingPlanBody {
    pub weekly_intensity: u8,
    pub team_focus: String,
}

/// Idempotent per 02_TECHNICAL_ARCHITECTURE.md ("Idempotency") and
/// 06_MULTIPLAYER_AND_NETWORK_PROTOCOL.md ("Ready barrier",
/// "MarkReady is idempotent"). No body needed beyond the envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkReadyBody {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum LiveMatchCommandKind {
    Substitute {
        player_out_id: String,
        player_in_id: String,
    },
    ChangeFormation {
        formation: String,
    },
    ChangeRoleDuty {
        player_id: String,
        role: String,
        duty: String,
    },
    SetTeamInstruction {
        key: String,
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyLiveMatchCommandBody {
    pub match_id: Uuid,
    pub command: LiveMatchCommandKind,
}
