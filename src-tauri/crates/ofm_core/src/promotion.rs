//! Promotion and relegation between the divisions of a domestic pyramid.
//!
//! A pyramid is an ordered set of league competitions (highest tier first).
//! After a season, the bottom clubs of each division swap with the top clubs of
//! the division below it. Only `participant_ids` are updated here; fixtures are
//! regenerated separately for the new season.

use std::collections::HashSet;

use domain::league::League;

/// Number of clubs that swap between two adjacent divisions, scaled to the
/// smaller division (roughly one slot per five clubs, at least one). A 20-club
/// division yields four; smaller leagues move fewer.
pub fn relegation_count(top_size: usize, bottom_size: usize) -> usize {
    (top_size.min(bottom_size) / 5).max(1)
}

/// Apply promotion/relegation across `divisions`, ordered highest tier first.
/// Swaps are computed from each division's final standings before any club
/// moves, so multi-tier pyramids resolve consistently.
pub fn apply_promotion_relegation(divisions: &mut [League]) {
    apply_promotion_relegation_with_playoff_winners(divisions, &std::collections::HashMap::new());
}

/// Apply promotion/relegation using ruleset-owned automatic slots and completed
/// promotion-playoff winners where they exist. Legacy/generated pyramids with
/// no explicit slots retain the generic scaled policy.
pub fn apply_promotion_relegation_with_playoff_winners(
    divisions: &mut [League],
    playoff_winners: &std::collections::HashMap<String, String>,
) {
    let tiers = divisions.len();
    if tiers < 2 {
        return;
    }

    // For each boundary i / i+1, the clubs leaving downward and upward.
    let mut relegated_at: Vec<Vec<String>> = vec![Vec::new(); tiers];
    let mut promoted_at: Vec<Vec<String>> = vec![Vec::new(); tiers];

    for i in 0..tiers - 1 {
        let upper_standings = divisions[i].sorted_standings();
        let lower_standings = divisions[i + 1].sorted_standings();
        if upper_standings.is_empty() || lower_standings.is_empty() {
            continue;
        }
        let upper_division = &divisions[i];
        let lower_division = &divisions[i + 1];
        let configured_promotions = usize::from(lower_division.rules.promotion_automatic_slots)
            + usize::from((lower_division.rules.promotion_playoff_slots >= 2) as u8);
        let count = if upper_division.rules.relegation_automatic_slots > 0 {
            usize::from(upper_division.rules.relegation_automatic_slots)
        } else if configured_promotions > 0 {
            configured_promotions
        } else {
            relegation_count(upper_division.participant_ids.len(), lower_division.participant_ids.len())
        };
        relegated_at[i] = upper_standings
            .iter()
            .rev()
            .take(count)
            .map(|entry| entry.team_id.clone())
            .collect();
        let automatic = if lower_division.rules.promotion_automatic_slots > 0 {
            usize::from(lower_division.rules.promotion_automatic_slots)
        } else {
            count
        };
        promoted_at[i] = lower_standings.iter().take(automatic).map(|entry| entry.team_id.clone()).collect();
        if lower_division.rules.promotion_playoff_slots >= 2 {
            let from = automatic as u32 + 1;
            let to = from + u32::from(lower_division.rules.promotion_playoff_slots) - 1;
            let playoff_id = format!("{}-playoff-{from}-{to}", divisions[i + 1].id);
            if let Some(winner) = playoff_winners.get(&playoff_id)
                && !promoted_at[i].contains(winner)
            {
                promoted_at[i].push(winner.clone());
            }
        }
        // A caller should only roll over after a configured playoff settles,
        // but keep the pyramid structurally sound for imported legacy saves.
        for entry in lower_standings {
            if promoted_at[i].len() >= count {
                break;
            }
            if !promoted_at[i].contains(&entry.team_id) {
                promoted_at[i].push(entry.team_id.clone());
            }
        }
        promoted_at[i].truncate(count);
    }

    for i in 0..tiers {
        let mut leaving: HashSet<&String> = HashSet::new();
        leaving.extend(relegated_at[i].iter()); // relegated to the division below
        if i >= 1 {
            leaving.extend(promoted_at[i - 1].iter()); // promoted to the division above
        }

        let mut new_participants: Vec<String> = divisions[i]
            .participant_ids
            .iter()
            .filter(|id| !leaving.contains(id))
            .cloned()
            .collect();

        if i >= 1 {
            new_participants.extend(relegated_at[i - 1].iter().cloned()); // arrivals from above
        }
        if i < tiers - 1 {
            new_participants.extend(promoted_at[i].iter().cloned()); // arrivals from below
        }

        divisions[i].participant_ids = new_participants;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::league::{League, StandingEntry};

    fn division(id: &str, priority: u32, standings: &[(&str, u32)]) -> League {
        let team_ids: Vec<String> = standings.iter().map(|(id, _)| id.to_string()).collect();
        let mut league = League::new(id.to_string(), id.to_string(), 2026, &team_ids);
        league.priority = priority;
        league.standings = standings
            .iter()
            .map(|(team, points)| {
                let mut entry = StandingEntry::new(team.to_string());
                entry.points = *points;
                entry
            })
            .collect();
        league
    }

    #[test]
    fn relegation_count_scales_with_division_size() {
        assert_eq!(relegation_count(20, 20), 4);
        assert_eq!(relegation_count(10, 10), 2);
        assert_eq!(relegation_count(8, 8), 1);
        assert_eq!(relegation_count(4, 4), 1);
        // Bound by the smaller division.
        assert_eq!(relegation_count(20, 6), 1);
    }

    #[test]
    fn apply_promotion_relegation_swaps_bottom_and_top_clubs() {
        // Top division: t1 best ... t6 worst. Second division: s1 best ... s6 worst.
        let mut divisions = vec![
            division(
                "top",
                0,
                &[
                    ("t1", 60),
                    ("t2", 50),
                    ("t3", 40),
                    ("t4", 30),
                    ("t5", 20),
                    ("t6", 10),
                ],
            ),
            division(
                "second",
                1,
                &[
                    ("s1", 60),
                    ("s2", 50),
                    ("s3", 40),
                    ("s4", 30),
                    ("s5", 20),
                    ("s6", 10),
                ],
            ),
        ];

        // 6-club divisions -> one up / one down.
        apply_promotion_relegation(&mut divisions);

        let top: HashSet<&String> = divisions[0].participant_ids.iter().collect();
        let second: HashSet<&String> = divisions[1].participant_ids.iter().collect();

        assert!(!top.contains(&"t6".to_string()), "worst top club relegated");
        assert!(top.contains(&"s1".to_string()), "best second club promoted");
        assert!(second.contains(&"t6".to_string()));
        assert!(!second.contains(&"s1".to_string()));
        // Sizes preserved.
        assert_eq!(divisions[0].participant_ids.len(), 6);
        assert_eq!(divisions[1].participant_ids.len(), 6);
    }

    #[test]
    fn apply_promotion_relegation_is_noop_for_single_division() {
        let mut divisions = vec![division("only", 0, &[("a", 10), ("b", 5)])];
        apply_promotion_relegation(&mut divisions);
        assert_eq!(
            divisions[0].participant_ids,
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn apply_promotion_relegation_skips_boundary_without_standings() {
        let mut top = division("top", 0, &[("t1", 30), ("t2", 10)]);
        top.standings.clear(); // never simulated
        let mut divisions = vec![top, division("second", 1, &[("s1", 30), ("s2", 10)])];

        apply_promotion_relegation(&mut divisions);

        // No standings to rank the top division, so nothing moves.
        assert_eq!(
            divisions[0].participant_ids,
            vec!["t1".to_string(), "t2".to_string()]
        );
        assert_eq!(
            divisions[1].participant_ids,
            vec!["s1".to_string(), "s2".to_string()]
        );
    }

    #[test]
    fn configured_playoff_winner_takes_the_final_promotion_slot() {
        let mut top = division(
            "top",
            0,
            &[("t1", 60), ("t2", 50), ("t3", 40), ("t4", 30), ("t5", 20), ("t6", 10)],
        );
        top.rules.relegation_automatic_slots = 2;
        let mut second = division(
            "second",
            1,
            &[("s1", 60), ("s2", 50), ("s3", 40), ("s4", 30), ("s5", 20), ("s6", 10)],
        );
        second.rules.promotion_automatic_slots = 1;
        second.rules.promotion_playoff_slots = 4;
        let mut divisions = vec![top, second];
        let winners = std::collections::HashMap::from([(
            "second-playoff-2-5".to_string(),
            "s4".to_string(),
        )]);

        apply_promotion_relegation_with_playoff_winners(&mut divisions, &winners);

        assert!(divisions[0].participant_ids.contains(&"s1".to_string()));
        assert!(divisions[0].participant_ids.contains(&"s4".to_string()));
        assert!(!divisions[0].participant_ids.contains(&"s2".to_string()));
        assert_eq!(divisions[0].participant_ids.len(), 6);
        assert_eq!(divisions[1].participant_ids.len(), 6);
    }

    #[test]
    fn twenty_structural_seasons_preserve_pyramid_membership_and_schedules() {
        let mut divisions = vec![
            division("tier-1", 0, &[("a", 40), ("b", 30), ("c", 20), ("d", 10)]),
            division("tier-2", 1, &[("e", 40), ("f", 30), ("g", 20), ("h", 10)]),
            division("tier-3", 2, &[("i", 40), ("j", 30), ("k", 20), ("l", 10)]),
        ];
        let all_clubs = divisions
            .iter()
            .flat_map(|division| division.participant_ids.iter().cloned())
            .collect::<HashSet<_>>();

        for season in 2027..=2046 {
            // Produce a deterministic complete table for every tier, rotating
            // the leader each year to exercise both directions of each boundary.
            for (tier, division) in divisions.iter_mut().enumerate() {
                let club_count = division.participant_ids.len();
                division
                    .participant_ids
                    .rotate_left((season as usize + tier) % club_count);
                division.standings = division
                    .participant_ids
                    .iter()
                    .enumerate()
                    .map(|(rank, team_id)| {
                        let mut entry = StandingEntry::new(team_id.clone());
                        entry.played = 6;
                        entry.won = (4usize.saturating_sub(rank)) as u32;
                        entry.lost = rank as u32;
                        entry.points = entry.won * 3;
                        entry
                    })
                    .collect();
            }

            apply_promotion_relegation(&mut divisions);
            let start = Utc
                .with_ymd_and_hms(season, 8, 1, 12, 0, 0)
                .single()
                .expect("valid season date");
            for division in &mut divisions {
                crate::schedule::regenerate_league_for_season(division, season as u32, start);
                assert_eq!(
                    division.fixtures.len(),
                    division.participant_ids.len() * (division.participant_ids.len() - 1)
                );
                for fixture in &division.fixtures {
                    assert_ne!(
                        fixture.home_team_id, fixture.away_team_id,
                        "a club cannot play itself"
                    );
                    assert!(division.participant_ids.contains(&fixture.home_team_id));
                    assert!(division.participant_ids.contains(&fixture.away_team_id));
                }
            }

            let current_clubs = divisions
                .iter()
                .flat_map(|division| division.participant_ids.iter().cloned())
                .collect::<Vec<_>>();
            assert_eq!(current_clubs.len(), all_clubs.len());
            assert_eq!(
                current_clubs.iter().collect::<HashSet<_>>().len(),
                all_clubs.len(),
                "club belongs to exactly one tier"
            );
            assert_eq!(current_clubs.into_iter().collect::<HashSet<_>>(), all_clubs);
        }
    }
}
