use serde::{Deserialize, Serialize};

/// Persist independent versions per 02_TECHNICAL_ARCHITECTURE.md
/// ("State/version rules"). A career records the versions it was created
/// with; a compatibility handshake compares them against what the running
/// app/server actually supports before letting a client/save proceed.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Wire protocol version. Bump on any breaking command/event/envelope
/// change. Additive, backward-compatible fields do not require a bump.
pub const PROTOCOL_VERSION: u32 = 1;

/// Per-career SQLite schema version (03_DATA_SNAPSHOT_AND_DATABASE.md,
/// "Career database model"). Every migration bumps this.
pub const SAVE_SCHEMA_VERSION: u32 = 1;

/// Schema version of a published real-world snapshot package
/// (03_DATA_SNAPSHOT_AND_DATABASE.md, "Snapshot states").
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Match engine version (05_MATCH_ENGINE_AND_BALANCE.md, "Engine version
/// policy"). Historical results keep the engine version that produced them;
/// this constant is only "the engine version new matches are simulated
/// with today".
pub const MATCH_ENGINE_VERSION: &str = "albion-engine-v1";

/// Rating model version (03_DATA_SNAPSHOT_AND_DATABASE.md, "Rating model").
/// Every generated attribute set records the model version that produced
/// it; this constant is "the model current imports/youth-gen use today".
pub const RATING_MODEL_VERSION: &str = "albion-rating-v1";

/// Bundled snapshot of every version a running app/server instance carries.
/// A ruleset is added when the runtime opens a particular career: it cannot
/// be a global constant because two open careers can legitimately use
/// different ruleset versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionSet {
    pub app_version: String,
    pub protocol_version: u32,
    pub save_schema_version: u32,
    pub snapshot_schema_version: u32,
    pub match_engine_version: String,
    pub rating_model_version: String,
    /// The season rules pinned by a career. It is optional so existing
    /// pre-Phase-1 metadata can be read and rejected non-destructively by a
    /// caller that requires a bound ruleset, instead of failing to parse.
    #[serde(default)]
    pub ruleset: Option<RulesetVersion>,
}

/// Identifies the exact versioned ruleset that governs a career. The id is
/// needed alongside its numeric version because different competition packs
/// can legitimately both start at version 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesetVersion {
    pub ruleset_id: String,
    pub ruleset_version: u32,
}

pub const CURRENT_VERSIONS: VersionSetStatic = VersionSetStatic {
    app_version: APP_VERSION,
    protocol_version: PROTOCOL_VERSION,
    save_schema_version: SAVE_SCHEMA_VERSION,
    snapshot_schema_version: SNAPSHOT_SCHEMA_VERSION,
    match_engine_version: MATCH_ENGINE_VERSION,
    rating_model_version: RATING_MODEL_VERSION,
};

/// A `const`-friendly mirror of [`VersionSet`] (which owns `String`s and so
/// cannot itself be a `const`). Convert with `.to_owned_set()` when a real
/// `VersionSet` is needed (e.g. to serialize into a `Hello` message).
pub struct VersionSetStatic {
    pub app_version: &'static str,
    pub protocol_version: u32,
    pub save_schema_version: u32,
    pub snapshot_schema_version: u32,
    pub match_engine_version: &'static str,
    pub rating_model_version: &'static str,
}

impl VersionSetStatic {
    pub fn to_owned_set(&self) -> VersionSet {
        VersionSet {
            app_version: self.app_version.to_string(),
            protocol_version: self.protocol_version,
            save_schema_version: self.save_schema_version,
            snapshot_schema_version: self.snapshot_schema_version,
            match_engine_version: self.match_engine_version.to_string(),
            rating_model_version: self.rating_model_version.to_string(),
            ruleset: None,
        }
    }
}

impl VersionSet {
    /// Bind a runtime version set to the exact ruleset selected when opening
    /// a career. The server includes this in its Hello handshake response.
    pub fn with_ruleset(mut self, ruleset: RulesetVersion) -> Self {
        self.ruleset = Some(ruleset);
        self
    }

    /// Strict compatibility: every field must match exactly. Good enough
    /// for v1 (private two-friends deployment where client/server ship
    /// together); can be relaxed to per-field policy later if needed.
    pub fn is_compatible_with(&self, other: &VersionSet) -> bool {
        self == other
    }
}
