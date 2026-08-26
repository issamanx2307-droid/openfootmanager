//! Project Albion wire protocol.
//!
//! Per `02_TECHNICAL_ARCHITECTURE.md` / `06_MULTIPLAYER_AND_NETWORK_PROTOCOL.md`:
//! typed serde commands/events/views, an explicit protocol_version for the
//! compatibility handshake, and stable error codes rather than raw strings.
//! This crate has no Tauri/HTTP/WebSocket/DB dependency — `albion_server`
//! and `albion-client` both depend on it as the shared source of truth for
//! what can be said over the wire.

mod envelope;
mod error;
mod versions;

pub mod command;
pub mod event;

pub use envelope::{Envelope, EnvelopePayload, MessageKind};
pub use error::{ErrorCode, ProtocolError};
pub use versions::{CURRENT_VERSIONS, RulesetVersion, VersionSet};

#[cfg(test)]
mod tests;
