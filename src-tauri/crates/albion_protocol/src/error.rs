use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable error codes for `CommandRejected` (06_MULTIPLAYER_AND_NETWORK_PROTOCOL.md,
/// "Server response model"). The client localizes these; the server never
/// sends raw English strings or backtraces over the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AuthInvalid,
    ProtocolIncompatible,
    StaleRevision,
    ClubAlreadyControlled,
    InvalidLineup,
    TransferWindowClosed,
    InsufficientTransferBudget,
    InsufficientWageBudget,
    RegistrationInvalid,
    MatchCommandNotAllowed,
    SaveCorrupt,
    SnapshotInvalid,
}

/// Wire-level error payload: a stable code plus structured parameters for
/// client-side localization/interpolation, never a pre-rendered message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: ErrorCode,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

impl ProtocolError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
}
