//! Canonical game mutations performed by the authoritative command lane.

use albion_protocol::command::{Command, SetStartingXiBody, SetTacticsBody, SetTrainingPlanBody};
use albion_protocol::ErrorCode;
use domain::team::{PlayStyle, TrainingFocus, TrainingIntensity};
use ofm_core::game::Game;
use ofm_core::player_rating::formation_slots;
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CanonicalCareer {
    game: Game,
}

impl CanonicalCareer {
    pub fn new(game: Game) -> Self {
        Self { game }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn manager_controls(&self, manager_id: Uuid, club_id: Uuid) -> bool {
        self.controlled_team_id(manager_id)
            .ok()
            .is_some_and(|team_id| team_id == club_id.to_string())
    }

    pub fn apply(&mut self, manager_id: Uuid, command: &Command) -> Result<Value, ErrorCode> {
        match command {
            Command::SetTactics(body) => self.set_tactics(manager_id, body),
            Command::SetStartingXi(body) => self.set_starting_xi(manager_id, body),
            Command::SetTrainingPlan(body) => self.set_training_plan(manager_id, body),
            _ => Err(ErrorCode::MatchCommandNotAllowed),
        }
    }

    fn controlled_team_id(&self, manager_id: Uuid) -> Result<String, ErrorCode> {
        let manager_id = manager_id.to_string();
        self.game
            .managers
            .iter()
            .find(|manager| manager.id == manager_id)
            .and_then(|manager| manager.team_id.clone())
            .or_else(|| {
                (self.game.manager.id == manager_id)
                    .then(|| self.game.manager.team_id.clone())
                    .flatten()
            })
            .ok_or(ErrorCode::AuthInvalid)
    }

    fn set_tactics(&mut self, manager_id: Uuid, body: &SetTacticsBody) -> Result<Value, ErrorCode> {
        if formation_slots(&body.formation).len() != 11 {
            return Err(ErrorCode::InvalidLineup);
        }
        let play_style = parse_play_style(&body.mentality).ok_or(ErrorCode::MatchCommandNotAllowed)?;
        let team_id = self.controlled_team_id(manager_id)?;
        let team = self
            .game
            .teams
            .iter_mut()
            .find(|team| team.id == team_id)
            .ok_or(ErrorCode::AuthInvalid)?;
        team.formation = body.formation.clone();
        team.play_style = play_style;
        Ok(json!({ "teamId": team.id, "formation": team.formation, "mentality": body.mentality }))
    }

    fn set_starting_xi(
        &mut self,
        manager_id: Uuid,
        body: &SetStartingXiBody,
    ) -> Result<Value, ErrorCode> {
        if body.player_ids.len() != 11
            || formation_slots(&body.formation).len() != 11
            || body.player_ids.iter().collect::<HashSet<_>>().len() != 11
        {
            return Err(ErrorCode::InvalidLineup);
        }
        let team_id = self.controlled_team_id(manager_id)?;
        let selected_ids: Vec<String> = body.player_ids.iter().map(Uuid::to_string).collect();
        let all_owned_and_healthy = selected_ids.iter().all(|player_id| {
            self.game.players.iter().any(|player| {
                player.id == *player_id && player.team_id.as_deref() == Some(team_id.as_str()) && player.injury.is_none()
            })
        });
        if !all_owned_and_healthy {
            return Err(ErrorCode::InvalidLineup);
        }
        let team = self
            .game
            .teams
            .iter_mut()
            .find(|team| team.id == team_id)
            .ok_or(ErrorCode::AuthInvalid)?;
        team.formation = body.formation.clone();
        team.starting_xi_ids = selected_ids;
        Ok(json!({ "teamId": team.id, "fixtureId": body.fixture_id, "startingXiSet": true }))
    }

    fn set_training_plan(
        &mut self,
        manager_id: Uuid,
        body: &SetTrainingPlanBody,
    ) -> Result<Value, ErrorCode> {
        let intensity = parse_training_intensity(body.weekly_intensity).ok_or(ErrorCode::MatchCommandNotAllowed)?;
        let focus = parse_training_focus(&body.team_focus).ok_or(ErrorCode::MatchCommandNotAllowed)?;
        let team_id = self.controlled_team_id(manager_id)?;
        let team = self
            .game
            .teams
            .iter_mut()
            .find(|team| team.id == team_id)
            .ok_or(ErrorCode::AuthInvalid)?;
        team.training_intensity = intensity;
        team.training_focus = focus;
        Ok(json!({ "teamId": team.id, "weeklyIntensity": body.weekly_intensity, "teamFocus": body.team_focus }))
    }
}

fn parse_play_style(value: &str) -> Option<PlayStyle> {
    Some(match value.to_ascii_lowercase().as_str() {
        "balanced" => PlayStyle::Balanced,
        "attacking" => PlayStyle::Attacking,
        "defensive" => PlayStyle::Defensive,
        "possession" => PlayStyle::Possession,
        "counter" => PlayStyle::Counter,
        "high_press" | "highpress" => PlayStyle::HighPress,
        _ => return None,
    })
}

fn parse_training_focus(value: &str) -> Option<TrainingFocus> {
    Some(match value.to_ascii_lowercase().as_str() {
        "physical" => TrainingFocus::Physical,
        "technical" => TrainingFocus::Technical,
        "tactical" => TrainingFocus::Tactical,
        "defending" => TrainingFocus::Defending,
        "attacking" => TrainingFocus::Attacking,
        "recovery" => TrainingFocus::Recovery,
        _ => return None,
    })
}

fn parse_training_intensity(value: u8) -> Option<TrainingIntensity> {
    Some(match value {
        0..=33 => TrainingIntensity::Low,
        34..=66 => TrainingIntensity::Medium,
        67..=100 => TrainingIntensity::High,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;
    use ofm_core::clock::GameClock;

    fn career() -> (CanonicalCareer, Uuid) {
        let manager_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();
        let mut manager = Manager::new(
            manager_id.to_string(), "Alex".into(), "Manager".into(), "1980-01-01".into(), "ENG".into(),
        );
        manager.hire(team_id.to_string());
        let mut team = Team::new(
            team_id.to_string(), "Albion".into(), "ALB".into(), "England".into(), "Albion".into(), "Ground".into(), 20_000,
        );
        team.manager_id = Some(manager_id.to_string());
        let game = Game::new(
            GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap()),
            manager,
            vec![team],
            vec![],
            vec![],
            vec![],
        );
        (CanonicalCareer::new(game), manager_id)
    }

    #[test]
    fn manager_can_mutate_only_its_canonical_team_tactics_and_training() {
        let (mut career, manager_id) = career();
        career.apply(manager_id, &Command::SetTactics(SetTacticsBody {
            formation: "4-3-3".into(), mentality: "high_press".into(),
        })).unwrap();
        career.apply(manager_id, &Command::SetTrainingPlan(SetTrainingPlanBody {
            weekly_intensity: 80, team_focus: "attacking".into(),
        })).unwrap();
        let team = &career.game().teams[0];
        assert_eq!(team.formation, "4-3-3");
        assert_eq!(team.play_style, PlayStyle::HighPress);
        assert_eq!(team.training_intensity, TrainingIntensity::High);
        assert_eq!(team.training_focus, TrainingFocus::Attacking);
    }
}
