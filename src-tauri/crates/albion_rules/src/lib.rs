//! Project Albion versioned competition and business rules.
//!
//! Per `02_TECHNICAL_ARCHITECTURE.md` / `03_DATA_SNAPSHOT_AND_DATABASE.md`:
//! season-specific rules live in versioned files under `data/rulesets/`
//! (e.g. `england_2026_27.yaml`), never as permanent hardcoded constants.
//! Game code validates a loaded ruleset rather than assuming one season's
//! rules remain true forever.

mod format;
mod load;
mod ruleset;
mod validate;

pub use format::{CompetitionFormat, TieBreaker};
pub use load::{RulesetLoadError, load_from_json_str, load_from_path, load_from_yaml_str};
pub use ruleset::{
    CompetitionRules, CupConfig, PlayoffConfig, PointsConfig, PromotionRelegationConfig,
    QualificationConfig, RegistrationConfig, RulesetManifest, SubstitutionConfig, TransferWindow,
};
pub use validate::{ValidationError, validate};

#[cfg(test)]
mod tests;
