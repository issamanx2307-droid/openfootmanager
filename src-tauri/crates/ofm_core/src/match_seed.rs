//! Deterministic seeds for fixtures simulated by the playable game.

use domain::league::Fixture;

/// Derive a stable seed from the fixture identity rather than runtime entropy.
/// A regenerated fixture has a new id/date and therefore a new match, while a
/// saved fixture always replays from the same football input seed.
pub fn fixture_seed(fixture: &Fixture) -> u64 {
    let source = format!(
        "albion-engine-v1|{}|{}|{}|{}",
        fixture.id, fixture.date, fixture.home_team_id, fixture.away_team_id
    );
    source
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus};

    fn fixture(id: &str) -> Fixture {
        Fixture {
            id: id.to_string(),
            matchday: 1,
            date: "2026-08-01".to_string(),
            home_team_id: "home".to_string(),
            away_team_id: "away".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
            ..Default::default()
        }
    }

    #[test]
    fn fixture_seed_is_stable_and_identity_sensitive() {
        let first = fixture("fixture-1");
        assert_eq!(fixture_seed(&first), fixture_seed(&first));

        let another = fixture("fixture-2");
        assert_ne!(fixture_seed(&first), fixture_seed(&another));
    }
}
