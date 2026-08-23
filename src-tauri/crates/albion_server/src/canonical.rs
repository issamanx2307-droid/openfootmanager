//! Canonical game mutations performed by the authoritative command lane.

use albion_protocol::command::{Command, SetStartingXiBody, SetTacticsBody, SetTrainingPlanBody};
use albion_protocol::ErrorCode;
use domain::team::{PlayStyle, TrainingFocus, TrainingIntensity};
use domain::league::FixtureStatus;
use ofm_core::game::Game;
use ofm_core::player_rating::formation_slots;
use ofm_core::live_match_manager::{create_live_match, LiveMatchSession, MatchMode};
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CanonicalCareer {
    game: Game,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advancement {
    AdvancedThrough { date: String },
    HumanFixture { fixture_id: Uuid, home_club_id: Uuid, away_club_id: Uuid },
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

    pub fn controlled_club(&self, manager_id: Uuid) -> Result<Uuid, ErrorCode> {
        let team_id = self.controlled_team_id(manager_id)?;
        Uuid::parse_str(&team_id)
            .map_err(|_| ErrorCode::AuthInvalid)
    }

    /// A deliberately narrow resync view. It contains only the reconnecting
    /// manager's club state and avoids serializing the canonical `Game` or
    /// hidden player data to every connected client.
    pub fn manager_dashboard(&self, manager_id: Uuid) -> Result<Value, ErrorCode> {
        let team_id = self.controlled_team_id(manager_id)?;
        let team = self
            .game
            .teams
            .iter()
            .find(|team| team.id == team_id)
            .ok_or(ErrorCode::AuthInvalid)?;
        Ok(json!({
            "currentDate": self.game.clock.current_date.format("%Y-%m-%d").to_string(),
            "club": {
                "id": team.id,
                "name": team.name,
                "finance": team.finance,
                "formation": team.formation,
                "playStyle": format!("{:?}", team.play_style),
            },
        }))
    }

    /// Advance AI-only dates until a controlled club reaches a scheduled
    /// fixture. The caller must create and coordinate that live match instead
    /// of passing it through the instant simulator.
    pub fn advance_until_human_blocker(&mut self, controlled_clubs: &HashSet<Uuid>) -> Advancement {
        for _ in 0..366 {
            let today = self.game.clock.current_date.format("%Y-%m-%d").to_string();
            if let Some(fixture) = self.game.competitions.iter().flat_map(|competition| competition.fixtures.iter()).find(|fixture| {
                fixture.status == FixtureStatus::Scheduled
                    && fixture.date == today
                    && (Uuid::parse_str(&fixture.home_team_id).ok().is_some_and(|id| controlled_clubs.contains(&id))
                        || Uuid::parse_str(&fixture.away_team_id).ok().is_some_and(|id| controlled_clubs.contains(&id)))
            }) {
                return Advancement::HumanFixture {
                    fixture_id: Uuid::parse_str(&fixture.id).unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, fixture.id.as_bytes())),
                    home_club_id: Uuid::parse_str(&fixture.home_team_id).unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, fixture.home_team_id.as_bytes())),
                    away_club_id: Uuid::parse_str(&fixture.away_team_id).unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, fixture.away_team_id.as_bytes())),
                };
            }
            ofm_core::turn::process_day(&mut self.game);
        }
        Advancement::AdvancedThrough {
            date: self.game.clock.current_date.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn open_live_match(
        &self,
        fixture_id: Uuid,
        controlled_clubs: &HashSet<Uuid>,
    ) -> Result<LiveMatchSession, String> {
        let (competition_index, fixture_index) = self
            .game
            .competitions
            .iter()
            .enumerate()
            .find_map(|(competition_index, competition)| {
                competition.fixtures.iter().enumerate().find_map(|(fixture_index, fixture)| {
                    let candidate = Uuid::parse_str(&fixture.id)
                        .unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, fixture.id.as_bytes()));
                    (candidate == fixture_id).then_some((competition_index, fixture_index))
                })
            })
            .ok_or_else(|| "be.error.liveMatch.fixtureNotFound".to_string())?;
        let fixture = &self.game.competitions[competition_index].fixtures[fixture_index];
        let home_club_id = Uuid::parse_str(&fixture.home_team_id)
            .unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, fixture.home_team_id.as_bytes()));
        let away_club_id = Uuid::parse_str(&fixture.away_team_id)
            .unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, fixture.away_team_id.as_bytes()));
        let mut match_game = self.game.clone();
        match_game.league = Some(self.game.competitions[competition_index].clone());
        let mut session = create_live_match(&match_game, fixture_index, MatchMode::Live, false)?;
        let mut human_sides = Vec::new();
        if controlled_clubs.contains(&home_club_id) {
            human_sides.push(engine::Side::Home);
        }
        if controlled_clubs.contains(&away_club_id) {
            human_sides.push(engine::Side::Away);
        }
        session.set_human_sides(human_sides);
        Ok(session)
    }

    pub fn finish_live_match(&mut self, mut session: LiveMatchSession) -> Result<serde_json::Value, String> {
        if !session.is_finished() {
            session.run_to_completion();
        }
        let fixture_index = session.fixture_index;
        let competition_id = session.competition_id.clone();
        let home_team_id = session.home_team_id.clone();
        let away_team_id = session.away_team_id.clone();
        let report = session.match_state.into_report();
        let competition_index = self
            .game
            .competitions
            .iter()
            .position(|competition| competition.id == competition_id)
            .ok_or_else(|| "be.error.liveMatch.fixtureNotFound".to_string())?;
        self.game.league = Some(self.game.competitions[competition_index].clone());
        ofm_core::turn::apply_match_report(
            &mut self.game,
            fixture_index,
            &home_team_id,
            &away_team_id,
            &report,
        );
        if let Some(updated) = self.game.league.clone() {
            self.game.competitions[competition_index] = updated;
        }
        self.game.sync_legacy_league();
        ofm_core::turn::finish_live_match_day(&mut self.game);
        Ok(json!({ "homeGoals": report.home_goals, "awayGoals": report.away_goals }))
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
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
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

    #[test]
    fn ready_advancement_stops_before_a_controlled_clubs_fixture() {
        let (mut career, _manager_id) = career();
        let team_id = career.game().teams[0].id.clone();
        let club_id = Uuid::parse_str(&team_id).unwrap();
        let fixture_id = Uuid::new_v4();
        let mut competition = League::default();
        competition.fixtures.push(Fixture {
            id: fixture_id.to_string(),
            competition_id: "league".into(),
            matchday: 1,
            date: career.game().clock.current_date.format("%Y-%m-%d").to_string(),
            home_team_id: team_id,
            away_team_id: Uuid::new_v4().to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        });
        career.game.competitions.push(competition);

        let outcome = career.advance_until_human_blocker(&HashSet::from([club_id]));
        assert!(matches!(outcome, Advancement::HumanFixture { fixture_id: actual, .. } if actual == fixture_id));
    }

    #[test]
    fn manager_dashboard_is_a_narrow_club_view() {
        let (career, manager_id) = career();
        let view = career.manager_dashboard(manager_id).unwrap();
        assert_eq!(view["club"]["name"], "Albion");
        assert!(view.get("players").is_none());
        assert!(view.get("managers").is_none());
    }
}
