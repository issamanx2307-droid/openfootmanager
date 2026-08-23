use serde::{Deserialize, Serialize};

/// Generic competition shapes. Ruleset config decides participants,
/// rounds, promotion/relegation, playoffs, cup format, etc.; this enum
/// only decides which structural family a competition belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionFormat {
    League,
    KnockoutCup,
    GroupThenKnockout,
    Playoff,
}

/// Ordered list of tie-break rules applied to league standings.
/// Ruleset-defined order, never hardcoded in `ofm_core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TieBreaker {
    GoalDifference,
    GoalsFor,
    HeadToHeadPoints,
    HeadToHeadGoalDifference,
    Wins,
    FairPlayPoints,
    Alphabetical,
}
