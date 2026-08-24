use std::collections::HashSet;
use std::fmt;

use crate::format::CompetitionFormat;
use crate::ruleset::RulesetManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyRulesetId,
    ZeroRulesetVersion,
    EmptySeason,
    NoCompetitions,
    DuplicateCompetitionId(String),
    ZeroParticipants(String),
    LeagueMissingPoints(String),
    LeagueMissingTieBreakers(String),
    NonPositivePoints(String),
    PromotionExceedsField {
        competition_id: String,
        slots: u32,
        participants: u32,
    },
    RelegationExceedsField {
        competition_id: String,
        slots: u32,
        participants: u32,
    },
    UnknownPromotionTarget {
        competition_id: String,
        target_id: String,
    },
    UnknownQualificationSource {
        competition_id: String,
        source_id: String,
    },
    CupMissingRounds(String),
    InvalidTransferWindow {
        competition_id: String,
        window_name: String,
    },
    InvalidSubstitutionConfig {
        competition_id: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRulesetId => write!(f, "ruleset_id must not be empty"),
            Self::ZeroRulesetVersion => write!(f, "ruleset_version must be greater than zero"),
            Self::EmptySeason => write!(f, "season must not be empty"),
            Self::NoCompetitions => write!(f, "ruleset defines zero competitions"),
            Self::DuplicateCompetitionId(id) => write!(f, "duplicate competition id: {id}"),
            Self::ZeroParticipants(id) => write!(f, "competition {id} has zero participant_clubs"),
            Self::LeagueMissingPoints(id) => {
                write!(f, "league competition {id} has no points config")
            }
            Self::LeagueMissingTieBreakers(id) => {
                write!(f, "league competition {id} has no tie_breakers")
            }
            Self::NonPositivePoints(id) => {
                write!(
                    f,
                    "competition {id} has a negative/zero-inconsistent points config"
                )
            }
            Self::PromotionExceedsField {
                competition_id,
                slots,
                participants,
            } => write!(
                f,
                "competition {competition_id} promotion slots ({slots}) exceed participant_clubs ({participants})"
            ),
            Self::RelegationExceedsField {
                competition_id,
                slots,
                participants,
            } => write!(
                f,
                "competition {competition_id} relegation slots ({slots}) exceed participant_clubs ({participants})"
            ),
            Self::UnknownPromotionTarget {
                competition_id,
                target_id,
            } => write!(
                f,
                "competition {competition_id} references unknown target_competition_id {target_id}"
            ),
            Self::UnknownQualificationSource {
                competition_id,
                source_id,
            } => write!(
                f,
                "competition {competition_id} references unknown source_competition_id {source_id}"
            ),
            Self::CupMissingRounds(id) => write!(f, "cup competition {id} has zero rounds"),
            Self::InvalidTransferWindow {
                competition_id,
                window_name,
            } => write!(
                f,
                "competition {competition_id} transfer window {window_name} has an invalid month/day"
            ),
            Self::InvalidSubstitutionConfig { competition_id } => write!(
                f,
                "competition {competition_id} has an invalid substitution configuration"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

fn is_valid_month_day(month: u8, day: u8) -> bool {
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        // Rulesets describe a season window, not a particular year. Allow
        // February 29 so the same ruleset remains valid in leap years.
        2 => 29,
        _ => return false,
    };

    (1..=days_in_month).contains(&day)
}

/// Validate a loaded ruleset. Errors are collected (not short-circuited) so
/// an operator/CI run sees every problem in one pass rather than fixing them
/// one at a time, matching 03_DATA_SNAPSHOT_AND_DATABASE.md's diff/validate
/// workflow philosophy.
pub fn validate(manifest: &RulesetManifest) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if manifest.ruleset_id.trim().is_empty() {
        errors.push(ValidationError::EmptyRulesetId);
    }
    if manifest.ruleset_version == 0 {
        errors.push(ValidationError::ZeroRulesetVersion);
    }
    if manifest.season.trim().is_empty() {
        errors.push(ValidationError::EmptySeason);
    }
    if manifest.competitions.is_empty() {
        errors.push(ValidationError::NoCompetitions);
    }

    let known_ids: HashSet<&str> = manifest
        .competitions
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    let mut seen_ids: HashSet<&str> = HashSet::new();

    for comp in &manifest.competitions {
        if !seen_ids.insert(comp.id.as_str()) {
            errors.push(ValidationError::DuplicateCompetitionId(comp.id.clone()));
        }
        if comp.participant_clubs == 0 {
            errors.push(ValidationError::ZeroParticipants(comp.id.clone()));
        }

        if comp.format == CompetitionFormat::League {
            match &comp.points {
                // A win must reward strictly more than a loss, and at least
                // as much as a draw; a draw must reward at least as much as
                // a loss. This rejects degenerate/inverted configs while
                // still allowing unusual-but-legal ones (e.g. draw == loss).
                Some(p) if p.win <= p.loss || p.win < p.draw || p.draw < p.loss => {
                    errors.push(ValidationError::NonPositivePoints(comp.id.clone()));
                }
                Some(_) => {}
                None => errors.push(ValidationError::LeagueMissingPoints(comp.id.clone())),
            }
            if comp.tie_breakers.is_empty() {
                errors.push(ValidationError::LeagueMissingTieBreakers(comp.id.clone()));
            }
        }

        if let Some(promo) = &comp.promotion {
            let slots = promo.automatic_slots + promo.playoff_slots;
            if slots > comp.participant_clubs {
                errors.push(ValidationError::PromotionExceedsField {
                    competition_id: comp.id.clone(),
                    slots,
                    participants: comp.participant_clubs,
                });
            }
            if !known_ids.contains(promo.target_competition_id.as_str()) {
                errors.push(ValidationError::UnknownPromotionTarget {
                    competition_id: comp.id.clone(),
                    target_id: promo.target_competition_id.clone(),
                });
            }
        }

        if let Some(releg) = &comp.relegation {
            let slots = releg.automatic_slots + releg.playoff_slots;
            if slots > comp.participant_clubs {
                errors.push(ValidationError::RelegationExceedsField {
                    competition_id: comp.id.clone(),
                    slots,
                    participants: comp.participant_clubs,
                });
            }
            if !known_ids.contains(releg.target_competition_id.as_str()) {
                errors.push(ValidationError::UnknownPromotionTarget {
                    competition_id: comp.id.clone(),
                    target_id: releg.target_competition_id.clone(),
                });
            }
        }

        if let Some(qual) = &comp.qualification
            && qual.source_ruleset_id.is_none()
            && !known_ids.contains(qual.source_competition_id.as_str())
        {
            errors.push(ValidationError::UnknownQualificationSource {
                competition_id: comp.id.clone(),
                source_id: qual.source_competition_id.clone(),
            });
        }

        if let Some(substitutions) = &comp.substitutions
            && (substitutions.max_substitutes == 0
                || substitutions.max_windows == 0
                || substitutions.max_windows > substitutions.max_substitutes)
        {
            errors.push(ValidationError::InvalidSubstitutionConfig {
                competition_id: comp.id.clone(),
            });
        }

        if matches!(
            comp.format,
            CompetitionFormat::KnockoutCup | CompetitionFormat::GroupThenKnockout
        ) {
            match &comp.cup {
                Some(cup) if cup.rounds == 0 => {
                    errors.push(ValidationError::CupMissingRounds(comp.id.clone()))
                }
                Some(_) => {}
                None => errors.push(ValidationError::CupMissingRounds(comp.id.clone())),
            }
        }

        for window in &comp.transfer_windows {
            let valid = is_valid_month_day(window.start_month, window.start_day)
                && is_valid_month_day(window.end_month, window.end_day);
            if !valid {
                errors.push(ValidationError::InvalidTransferWindow {
                    competition_id: comp.id.clone(),
                    window_name: window.name.clone(),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
