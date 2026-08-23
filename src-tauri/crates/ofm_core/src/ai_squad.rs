//! Last-resort squad continuity for AI-controlled clubs.
//!
//! Normal transfer-market evaluation remains responsible for improving squads.
//! This module only prevents a club from becoming unable to play after a batch
//! of contracts expires or transfers completes.

use crate::contract_wage_policy::wage_policy_allows_projection;
use crate::game::Game;
use chrono::Months;
use domain::player::{PlayerMovementEntry, PlayerMovementKind, Position};

const MINIMUM_SENIOR_SQUAD: usize = 16;
const MINIMUM_ANNUAL_WAGE: i64 = 500;

/// Give AI clubs a legal, position-balanced squad when normal market activity
/// leaves them short. The user-managed club is deliberately never changed.
pub fn ensure_ai_club_squads(game: &mut Game) {
    let user_team_id = game.manager.team_id.clone();
    let team_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| Some(&team.id) != user_team_id.as_ref())
        .map(|team| team.id.clone())
        .collect();

    for team_id in team_ids {
        replenish_team(game, &team_id);
    }
}

fn replenish_team(game: &mut Game, team_id: &str) {
    let Some(team) = game.teams.iter().find(|team| team.id == team_id).cloned() else {
        return;
    };

    let mut group_counts = squad_group_counts(game, team_id);
    let mut roster_size = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .count();
    let targets = [
        (Position::Goalkeeper, 2usize),
        (Position::Defender, 5usize),
        (Position::Midfielder, 5usize),
        (Position::Forward, 4usize),
    ];

    for (group, target) in targets {
        while group_counts[group_index(&group)] < target {
            if !sign_best_free_agent(game, &team, &group, roster_size) {
                return;
            }
            group_counts[group_index(&group)] += 1;
            roster_size += 1;
        }
    }

    while roster_size < MINIMUM_SENIOR_SQUAD {
        if !sign_best_free_agent(game, &team, &Position::Midfielder, roster_size) {
            return;
        }
        roster_size += 1;
    }
}

fn sign_best_free_agent(
    game: &mut Game,
    team: &domain::team::Team,
    required_group: &Position,
    roster_size: usize,
) -> bool {
    let current_bill: i64 = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team.id.as_str()))
        .map(|player| i64::from(player.wage))
        .sum::<i64>()
        + game
            .staff
            .iter()
            .filter(|staff_member| staff_member.team_id.as_deref() == Some(team.id.as_str()))
            .map(|staff_member| i64::from(staff_member.wage))
            .sum::<i64>();
    let remaining_slots = (MINIMUM_SENIOR_SQUAD.saturating_sub(roster_size)).max(1) as i64;
    let offered_wage = (team.wage_budget.saturating_sub(current_bill) / remaining_slots)
        .max(MINIMUM_ANNUAL_WAGE)
        .min(u32::MAX as i64) as u32;

    if !wage_policy_allows_projection(team, current_bill, current_bill + i64::from(offered_wage)) {
        return false;
    }

    let candidate_index = game
        .players
        .iter()
        .enumerate()
        .filter(|(_, player)| {
            !player.retired
                && player.team_id.is_none()
                && player.position.to_group_position() == *required_group
        })
        .max_by_key(|(_, player)| player_quality_key(player))
        .map(|(index, _)| index)
        .or_else(|| {
            game.players
                .iter()
                .enumerate()
                .filter(|(_, player)| !player.retired && player.team_id.is_none())
                .max_by_key(|(_, player)| player_quality_key(player))
                .map(|(index, _)| index)
        });

    let Some(candidate_index) = candidate_index else {
        return false;
    };
    let current_date = game.clock.current_date.date_naive();
    let Some(contract_end) = current_date.checked_add_months(Months::new(12)) else {
        return false;
    };
    let today = current_date.format("%Y-%m-%d").to_string();
    let player = &mut game.players[candidate_index];
    player.team_id = Some(team.id.clone());
    player.contract_end = Some(contract_end.format("%Y-%m-%d").to_string());
    player.wage = offered_wage;
    player.transfer_listed = false;
    player.loan_listed = false;
    player.transfer_offers.clear();
    player.loan_offers.clear();
    player.movement_history.push(PlayerMovementEntry {
        date: today,
        kind: PlayerMovementKind::FreeAgentSigning,
        from_team_id: None,
        from_team_name: None,
        to_team_id: Some(team.id.clone()),
        to_team_name: Some(team.name.clone()),
        fee: None,
        loan_end_date: None,
    });
    true
}

fn squad_group_counts(game: &Game, team_id: &str) -> [usize; 4] {
    let mut counts = [0; 4];
    for player in &game.players {
        if player.team_id.as_deref() == Some(team_id) {
            counts[group_index(&player.position.to_group_position())] += 1;
        }
    }
    counts
}

fn group_index(position: &Position) -> usize {
    match position.to_group_position() {
        Position::Goalkeeper => 0,
        Position::Defender => 1,
        Position::Midfielder => 2,
        Position::Forward => 3,
        _ => unreachable!("group positions are exhaustive"),
    }
}

fn player_quality_key(player: &domain::player::Player) -> (u8, u64, String) {
    (player.ovr, player.market_value, player.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_every_specialist_position() {
        assert_eq!(group_index(&Position::RightBack), 1);
        assert_eq!(group_index(&Position::AttackingMidfielder), 2);
        assert_eq!(group_index(&Position::Striker), 3);
    }
}
