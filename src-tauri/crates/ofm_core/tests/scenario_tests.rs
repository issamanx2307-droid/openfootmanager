//! End-to-end gameplay scenarios.
//!
//! These tests drive the real game pipeline (`turn::process_day`) over long
//! stretches of game time and assert *invariants* that must hold for any
//! season, regardless of how matches happen to play out. They are deliberately
//! outcome-independent: a match-engine balance change should never break them.
//!
//! The starting world is built from an explicit seed (`make_scenario_game`), so
//! the initial state is fully reproducible. The season *trajectory* is not yet
//! deterministic (the match engine and several turn subsystems still draw from
//! ambient randomness), which is why assertions check properties rather than
//! exact values. See the scenario-test notes in the PR for the path to full
//! determinism.

use chrono::{Datelike, TimeZone, Utc};
use domain::league::FixtureStatus;
use domain::manager::Manager;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::turn;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Fixture builder
// ---------------------------------------------------------------------------

/// Build a fully playable game from generated world data, with the manager
/// assigned to the first team in the world. Starts on 2026-07-01 (season start).
///
/// The world is generated from `seed`, so the starting state is reproducible:
/// a failure in CI can be replayed locally by running with the same seed.
fn make_scenario_game(seed: u64) -> Game {
    // A small, reproducible world keeps the full-season simulation fast; the
    // invariants under test hold for any world size.
    let world = generate_world_data_seeded_with(seed, &WorldGenConfig::compact(), &DefinitionSources::embedded_only());

    let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let clock = GameClock::new(start);

    let first_team = world
        .teams
        .first()
        .expect("generated world must have at least one team");

    let mut manager = Manager::new(
        "scenario-mgr".to_string(),
        "Scenario".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(first_team.id.clone());

    let team_ids: Vec<String> = world.teams.iter().map(|t| t.id.clone()).collect();

    let mut game = Game::new(
        clock,
        manager,
        world.teams,
        world.players,
        world.staff,
        vec![],
    );

    game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());

    repair_opening_youth_academies(&mut game);

    game.league = Some(ofm_core::schedule::generate_league(
        "Scenario League",
        2026,
        &team_ids,
        start,
    ));

    ofm_core::season_context::refresh_game_context(&mut game);

    game
}

// ---------------------------------------------------------------------------
// Driver helpers
// ---------------------------------------------------------------------------

/// Advance the game by `days`, processing one turn per day. Panics in
/// `process_day` are re-raised with the day index so failures are locatable.
fn advance_days(game: &mut Game, days: usize) {
    for day in 0..days {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            turn::process_day(game);
        }));
        if result.is_err() {
            panic!("process_day panicked on day {day}");
        }
    }
}

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------

/// Assert the structural invariants that must hold for any game state, whatever
/// the random outcomes were. Called repeatedly while a scenario runs.
fn assert_game_invariants(game: &Game) {
    let team_ids: HashSet<&str> = game.teams.iter().map(|t| t.id.as_str()).collect();

    // The world is never silently emptied.
    assert!(!game.teams.is_empty(), "no teams remain");
    assert!(!game.players.is_empty(), "no players remain");

    // Referential integrity: anyone assigned to a team points at a real team.
    for player in &game.players {
        if let Some(team_id) = player.team_id.as_deref() {
            assert!(
                team_ids.contains(team_id),
                "player {} references unknown team {team_id}",
                player.id
            );
        }
    }
    for member in &game.staff {
        if let Some(team_id) = member.team_id.as_deref() {
            assert!(
                team_ids.contains(team_id),
                "staff {} references unknown team {team_id}",
                member.id
            );
        }
    }

    // Finances stay in a sane range (guards against wraparound / overflow bugs).
    let limit = i64::MAX / 2;
    for team in &game.teams {
        assert!(
            team.finance.abs() < limit,
            "team {} finance out of sane range: {}",
            team.id,
            team.finance
        );
    }

    if let Some(league) = &game.league {
        // Every standings row maps to a real team, exactly once.
        let mut seen = HashSet::new();
        for row in &league.standings {
            assert!(
                team_ids.contains(row.team_id.as_str()),
                "standings row references unknown team {}",
                row.team_id
            );
            assert!(seen.insert(row.team_id.as_str()), "duplicate standings row");

            // Played games are accounted for, and points follow 3-1-0 scoring.
            assert_eq!(
                row.played,
                row.won + row.drawn + row.lost,
                "team {} played != W+D+L",
                row.team_id
            );
            assert_eq!(
                row.points,
                row.won * 3 + row.drawn,
                "team {} points != 3*W + D",
                row.team_id
            );
        }

        // Each played match adds one game to two teams, so the total is even,
        // and every goal scored by someone is conceded by someone else.
        let total_played: u32 = league.standings.iter().map(|r| r.played).sum();
        assert!(total_played.is_multiple_of(2), "total games played is odd");

        let goals_for: u32 = league.standings.iter().map(|r| r.goals_for).sum();
        let goals_against: u32 = league.standings.iter().map(|r| r.goals_against).sum();
        assert_eq!(
            goals_for, goals_against,
            "league goals for != goals against"
        );

        // A finished fixture must carry a result.
        for fixture in &league.fixtures {
            if fixture.status == FixtureStatus::Completed {
                assert!(
                    fixture.result.is_some(),
                    "completed fixture {} has no result",
                    fixture.id
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

/// The same seed must always produce the same starting world. This is the
/// foundation every reproducible scenario relies on.
#[test]
fn world_generation_is_reproducible() {
    let a = make_scenario_game(42);
    let b = make_scenario_game(42);

    assert_eq!(a.teams.len(), b.teams.len());
    assert_eq!(a.players.len(), b.players.len());

    // Compare the rng-derived content (attributes, finances, nationalities).
    // IDs are random UUIDs and are deliberately excluded.
    let fingerprint = |g: &Game| -> (i64, i64, Vec<String>) {
        let attr_sum: i64 = g
            .players
            .iter()
            .map(|p| {
                let a = &p.attributes;
                a.pace as i64 + a.passing as i64 + a.shooting as i64 + a.tackling as i64
            })
            .sum();
        let finance_sum: i64 = g
            .teams
            .iter()
            .map(|t| t.finance + t.reputation as i64)
            .sum();
        let nationalities: Vec<String> = g.players.iter().map(|p| p.nationality.clone()).collect();
        (attr_sum, finance_sum, nationalities)
    };

    assert_eq!(
        fingerprint(&a),
        fingerprint(&b),
        "same seed must produce an identical starting world"
    );
}

/// Two different seeds should produce different worlds (the seed actually
/// drives generation, rather than being ignored).
#[test]
fn different_seeds_produce_different_worlds() {
    let a = make_scenario_game(1);
    let b = make_scenario_game(2);

    let nationalities =
        |g: &Game| -> Vec<String> { g.players.iter().map(|p| p.nationality.clone()).collect() };

    assert_ne!(
        nationalities(&a),
        nationalities(&b),
        "different seeds should produce different worlds"
    );
}

/// Drive a full season day by day. Asserts no panic on any day, and that the
/// game-state invariants hold throughout and at the end.
#[test]
fn full_season_holds_invariants() {
    let mut game = make_scenario_game(1);

    assert_game_invariants(&game);

    // A season's fixtures span roughly Aug-May; 365 days covers it with margin
    // and rolls into the next season, exercising the season-rollover path too.
    for _ in 0..(365 / 30) {
        advance_days(&mut game, 30);
        assert_game_invariants(&game);
    }
    advance_days(&mut game, 365 % 30);
    assert_game_invariants(&game);

    // By now matches have been played, so standings should have moved.
    let games_played: u32 = game
        .league
        .as_ref()
        .map(|l| l.standings.iter().map(|r| r.played).sum())
        .unwrap_or(0);
    assert!(
        games_played > 0,
        "a full season should have played some matches"
    );
}

/// Phase-12 soak gate: ten calendar years through the real daily pipeline.
/// Checking every month keeps the failure location useful without weakening
/// the long-run coverage to a final-state-only assertion.
#[test]
fn ten_season_soak_preserves_game_invariants() {
    let mut game = make_scenario_game(10_042);
    assert_game_invariants(&game);

    for month in 0..(10 * 12) {
        advance_days(&mut game, 31);
        assert_game_invariants(&game);
        assert!(game.clock.current_date.year() >= 2026, "clock regressed in soak month {month}");
    }
}

/// AI clubs must remain operational over multiple seasons: every non-user club
/// keeps a manager, enough players to field a side, finite finances, and its
/// own tactical identity. This exercises the daily AI training, recruitment,
/// manager-satisfaction and vacancy-replacement paths together.
#[test]
fn multi_season_ai_clubs_remain_operational_and_distinct() {
    let mut game = make_scenario_game(17);
    ofm_core::ai_hiring::seed_ai_managers(&mut game);
    let user_team_id = game.manager.team_id.clone().expect("scenario user team");
    let styles = [
        domain::team::PlayStyle::Attacking,
        domain::team::PlayStyle::Defensive,
        domain::team::PlayStyle::Possession,
        domain::team::PlayStyle::Counter,
        domain::team::PlayStyle::HighPress,
    ];
    for (index, team) in game.teams.iter_mut().enumerate() {
        if team.id != user_team_id {
            team.play_style = styles[index % styles.len()].clone();
        }
    }

    // Drive two full seasons plus a small rollover margin.
    advance_days(&mut game, 737);
    assert_game_invariants(&game);

    let non_user_teams: Vec<_> = game
        .teams
        .iter()
        .filter(|team| team.id != user_team_id)
        .collect();
    assert!(!non_user_teams.is_empty(), "scenario needs AI clubs");
    for team in &non_user_teams {
        assert!(team.manager_id.is_some(), "{} has no AI manager", team.id);
        assert!(
            game.managers
                .iter()
                .any(|manager| manager.id == *team.manager_id.as_ref().unwrap()
                    && manager.team_id.as_deref() == Some(team.id.as_str())),
            "{} manager record is not persistent",
            team.id
        );
        let available = game
            .players
            .iter()
            .filter(|player| {
                player.team_id.as_deref() == Some(team.id.as_str()) && player.injury.is_none()
            })
            .count();
        assert!(available >= 11, "{} cannot field a legal XI", team.id);
        assert!(
            team.finance.abs() < i64::MAX / 2,
            "{} finance overflow",
            team.id
        );
    }

    let styles_after: HashSet<_> = non_user_teams
        .iter()
        .map(|team| format!("{:?}", team.play_style))
        .collect();
    assert!(styles_after.len() > 1, "AI tactical identities collapsed");
}
