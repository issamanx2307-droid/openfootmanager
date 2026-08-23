use crate::{
    CompetitionFormat, CompetitionRules, PointsConfig, RulesetManifest, TieBreaker,
    load_from_json_str, load_from_path, load_from_yaml_str, validate,
};
use std::path::PathBuf;

fn repo_data_path(file: &str) -> PathBuf {
    // Crate is src-tauri/crates/albion_rules; data lives at repo root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/rulesets")
        .join(file)
}

fn minimal_valid_league() -> RulesetManifest {
    RulesetManifest {
        ruleset_id: "test-ruleset-v1".into(),
        ruleset_version: 1,
        season: "2026/27".into(),
        competitions: vec![CompetitionRules {
            id: "top-flight".into(),
            name: "Top Flight".into(),
            format: CompetitionFormat::League,
            tier: 1,
            participant_clubs: 20,
            points: Some(PointsConfig {
                win: 3,
                draw: 1,
                loss: 0,
            }),
            tie_breakers: vec![TieBreaker::GoalDifference],
            promotion: None,
            relegation: None,
            playoff: None,
            registration: None,
            substitutions: None,
            cup: None,
            qualification: None,
            transfer_windows: vec![],
        }],
    }
}

#[test]
fn minimal_manifest_round_trips_through_yaml() {
    let manifest = minimal_valid_league();
    let yaml = serde_yaml::to_string(&manifest).unwrap();
    let parsed = load_from_yaml_str(&yaml).unwrap();
    assert_eq!(parsed.ruleset_id, manifest.ruleset_id);
    assert_eq!(parsed.competitions.len(), 1);
}

#[test]
fn minimal_manifest_round_trips_through_json() {
    let manifest = minimal_valid_league();
    let json = serde_json::to_string(&manifest).unwrap();
    let parsed = load_from_json_str(&json).unwrap();
    assert_eq!(parsed.ruleset_id, manifest.ruleset_id);
}

#[test]
fn minimal_manifest_passes_validation() {
    assert!(validate(&minimal_valid_league()).is_ok());
}

#[test]
fn empty_ruleset_id_is_rejected() {
    let mut manifest = minimal_valid_league();
    manifest.ruleset_id = "".into();
    let errors = validate(&manifest).unwrap_err();
    assert!(errors.contains(&crate::ValidationError::EmptyRulesetId));
}

#[test]
fn zero_ruleset_version_is_rejected() {
    let mut manifest = minimal_valid_league();
    manifest.ruleset_version = 0;
    let errors = validate(&manifest).unwrap_err();
    assert!(errors.contains(&crate::ValidationError::ZeroRulesetVersion));
}

#[test]
fn impossible_transfer_window_calendar_date_is_rejected() {
    let mut manifest = minimal_valid_league();
    manifest.competitions[0]
        .transfer_windows
        .push(crate::TransferWindow {
            name: "impossible".into(),
            start_month: 2,
            start_day: 31,
            end_month: 3,
            end_day: 1,
        });

    let errors = validate(&manifest).unwrap_err();
    assert!(errors.iter().any(|error| matches!(
        error,
        crate::ValidationError::InvalidTransferWindow {
            competition_id,
            window_name,
        } if competition_id == "top-flight" && window_name == "impossible"
    )));
}

#[test]
fn substitution_windows_cannot_exceed_allowed_substitutes() {
    let mut manifest = minimal_valid_league();
    manifest.competitions[0].substitutions = Some(crate::SubstitutionConfig {
        max_substitutes: 3,
        max_windows: 4,
        half_time_does_not_count_as_window: true,
        concussion_substitutes: 0,
    });

    let errors = validate(&manifest).unwrap_err();
    assert!(errors.iter().any(|error| matches!(
        error,
        crate::ValidationError::InvalidSubstitutionConfig { competition_id }
            if competition_id == "top-flight"
    )));
}

#[test]
fn duplicate_competition_ids_are_rejected() {
    let mut manifest = minimal_valid_league();
    let dup = manifest.competitions[0].id.clone();
    manifest.competitions.push(manifest.competitions[0].clone());
    let errors = validate(&manifest).unwrap_err();
    assert!(errors.contains(&crate::ValidationError::DuplicateCompetitionId(dup)));
}

#[test]
fn promotion_slots_exceeding_participants_is_rejected() {
    let mut manifest = minimal_valid_league();
    manifest.competitions.push(CompetitionRules {
        id: "second-flight".into(),
        name: "Second Flight".into(),
        format: CompetitionFormat::League,
        tier: 2,
        participant_clubs: 20,
        points: Some(PointsConfig {
            win: 3,
            draw: 1,
            loss: 0,
        }),
        tie_breakers: vec![TieBreaker::GoalDifference],
        promotion: Some(crate::PromotionRelegationConfig {
            automatic_slots: 30,
            playoff_slots: 0,
            target_competition_id: "top-flight".into(),
        }),
        relegation: None,
        playoff: None,
        registration: None,
        substitutions: None,
        cup: None,
        qualification: None,
        transfer_windows: vec![],
    });
    let errors = validate(&manifest).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, crate::ValidationError::PromotionExceedsField { .. }))
    );
}

#[test]
fn unknown_promotion_target_is_rejected() {
    let mut manifest = minimal_valid_league();
    manifest.competitions[0].promotion = Some(crate::PromotionRelegationConfig {
        automatic_slots: 1,
        playoff_slots: 0,
        target_competition_id: "does-not-exist".into(),
    });
    let errors = validate(&manifest).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, crate::ValidationError::UnknownPromotionTarget { .. }))
    );
}

#[test]
fn england_ruleset_file_loads_and_validates() {
    let path = repo_data_path("england_2026_27.yaml");
    let manifest =
        load_from_path(&path).unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
    assert_eq!(manifest.ruleset_id, "england-2026-27-v1");
    assert_eq!(manifest.competitions.len(), 8);
    validate(&manifest)
        .unwrap_or_else(|errors| panic!("england_2026_27.yaml failed validation: {errors:?}"));
}

#[test]
fn continental_ruleset_file_loads_and_validates() {
    let path = repo_data_path("continental_2026_27.yaml");
    let manifest =
        load_from_path(&path).unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
    assert_eq!(manifest.ruleset_id, "continental-2026-27-v1");
    assert_eq!(manifest.competitions.len(), 3);
    validate(&manifest)
        .unwrap_or_else(|errors| panic!("continental_2026_27.yaml failed validation: {errors:?}"));
}

#[test]
fn england_pyramid_promotion_relegation_chain_is_internally_consistent() {
    let manifest = load_from_path(&repo_data_path("england_2026_27.yaml")).unwrap();
    let by_id = |id: &str| manifest.competitions.iter().find(|c| c.id == id).unwrap();

    // Every tiered league's relegation target should itself exist as a
    // league one tier below (or, for League Two, the National League).
    for id in ["premier-league", "championship", "league-one", "league-two"] {
        let comp = by_id(id);
        let releg = comp
            .relegation
            .as_ref()
            .unwrap_or_else(|| panic!("{id} has no relegation config"));
        let target = by_id(&releg.target_competition_id);
        assert!(
            target.tier > comp.tier,
            "{id} (tier {}) should relegate to a lower tier, got {} (tier {})",
            comp.tier,
            target.name,
            target.tier
        );
    }
}
