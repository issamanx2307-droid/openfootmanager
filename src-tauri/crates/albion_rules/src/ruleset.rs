use serde::{Deserialize, Serialize};

use crate::format::{CompetitionFormat, TieBreaker};

/// Top-level ruleset document, one per season per file
/// (e.g. `data/rulesets/england_2026_27.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesetManifest {
    pub ruleset_id: String,
    /// Bumped whenever this ruleset's content changes in a way that could
    /// affect a running career; careers pin the ruleset_version they were
    /// created with (03_DATA_SNAPSHOT_AND_DATABASE.md, "Versioning").
    pub ruleset_version: u32,
    pub season: String,
    #[serde(default)]
    pub competitions: Vec<CompetitionRules>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionRules {
    pub id: String,
    pub name: String,
    pub format: CompetitionFormat,
    /// English pyramid tier for promotion/relegation ordering. 0 for
    /// competitions outside the domestic league ladder (cups, continental).
    #[serde(default)]
    pub tier: u8,
    pub participant_clubs: u32,
    #[serde(default)]
    pub points: Option<PointsConfig>,
    #[serde(default)]
    pub tie_breakers: Vec<TieBreaker>,
    #[serde(default)]
    pub promotion: Option<PromotionRelegationConfig>,
    #[serde(default)]
    pub relegation: Option<PromotionRelegationConfig>,
    #[serde(default)]
    pub playoff: Option<PlayoffConfig>,
    #[serde(default)]
    pub registration: Option<RegistrationConfig>,
    #[serde(default)]
    pub substitutions: Option<SubstitutionConfig>,
    #[serde(default)]
    pub cup: Option<CupConfig>,
    #[serde(default)]
    pub qualification: Option<QualificationConfig>,
    #[serde(default)]
    pub transfer_windows: Vec<TransferWindow>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PointsConfig {
    pub win: u32,
    pub draw: u32,
    pub loss: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionRelegationConfig {
    /// Clubs moving automatically (no playoff), ranked from the relevant end
    /// of the table.
    pub automatic_slots: u32,
    /// Clubs additionally entering a promotion/relegation playoff, if any.
    #[serde(default)]
    pub playoff_slots: u32,
    /// id of the CompetitionRules this promotes into / relegates from.
    pub target_competition_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayoffConfig {
    pub participants: u32,
    pub two_legged_semis: bool,
    pub final_at_neutral_venue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationConfig {
    pub max_squad_size: u32,
    #[serde(default)]
    pub min_homegrown: u32,
    #[serde(default)]
    pub max_non_homegrown: Option<u32>,
    #[serde(default)]
    pub loan_ins_allowed: u32,
    #[serde(default)]
    pub loan_outs_allowed: u32,
}

/// Competition-specific substitution rules. Keeping these in the ruleset
/// avoids baking a particular season's allowance into match logic.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SubstitutionConfig {
    pub max_substitutes: u8,
    pub max_windows: u8,
    #[serde(default)]
    pub half_time_does_not_count_as_window: bool,
    #[serde(default)]
    pub concussion_substitutes: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CupConfig {
    pub rounds: u32,
    pub extra_time: bool,
    pub penalties_after_extra_time: bool,
    #[serde(default)]
    pub seeded_draw: bool,
    #[serde(default)]
    pub two_legged_from_round: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualificationConfig {
    /// Optional ruleset containing `source_competition_id`. Omit this when
    /// the qualifying competition lives in the same manifest.
    #[serde(default)]
    pub source_ruleset_id: Option<String>,
    /// id of the domestic CompetitionRules that feeds this one.
    pub source_competition_id: String,
    /// How many finishing positions from the source competition qualify.
    pub slots_from_source: u32,
}

/// Transfer/registration window, expressed as month-day pairs so a window
/// can span a year boundary (e.g. summer window into a new season).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferWindow {
    pub name: String,
    pub start_month: u8,
    pub start_day: u8,
    pub end_month: u8,
    pub end_day: u8,
}
