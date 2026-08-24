//! Canonical game mutations performed by the authoritative command lane.

use albion_protocol::command::{Command, RespondTransferOfferBody, SetStartingXiBody, SetTacticsBody, SetTrainingPlanBody, SubmitContractOfferBody, SubmitTransferBidBody, TransferOfferResponse};
use albion_protocol::ErrorCode;
use chrono::Datelike;
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
        let next_fixture = self.game.competitions.iter()
            .flat_map(|competition| competition.fixtures.iter().map(move |fixture| (competition, fixture)))
            .filter(|(_, fixture)| fixture.status == FixtureStatus::Scheduled
                && (fixture.home_team_id == team_id || fixture.away_team_id == team_id))
            .min_by(|(_, left), (_, right)| left.date.cmp(&right.date))
            .map(|(competition, fixture)| {
                let team_name = |id: &str| self.game.teams.iter()
                    .find(|candidate| candidate.id == id)
                    .map(|candidate| candidate.name.clone())
                    .unwrap_or_else(|| id.to_owned());
                json!({
                    "id": fixture.id,
                    "date": fixture.date,
                    "competition": competition.name,
                    "homeTeam": team_name(&fixture.home_team_id),
                    "awayTeam": team_name(&fixture.away_team_id),
                })
            });
        let squad = self.game.players.iter()
            .filter(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            .map(|player| json!({
                "id": player.id,
                "name": player.match_name,
                "position": format!("{:?}", player.position),
                "condition": player.condition,
                "injured": player.injury.is_some(),
            }))
            .collect::<Vec<_>>();
        let inbox = self.game.messages.iter()
            .filter(|message| message.context.team_id.as_deref().is_none_or(|id| id == team_id))
            .rev()
            .take(5)
            .map(|message| json!({
                "id": message.id,
                "subject": message.subject,
                "sender": message.sender,
                "date": message.date,
                "read": message.read,
                "priority": format!("{:?}", message.priority),
            }))
            .collect::<Vec<_>>();
        let incoming_transfer_offers = self.game.players.iter()
            .filter(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            .flat_map(|player| player.transfer_offers.iter()
                .filter(|offer| offer.status == domain::player::TransferOfferStatus::Pending)
                .map(|offer| {
                    let buyer_name = self.game.teams.iter()
                        .find(|candidate| candidate.id == offer.from_team_id)
                        .map(|candidate| candidate.name.clone())
                        .unwrap_or_else(|| offer.from_team_id.clone());
                    json!({
                        "offerId": offer.id,
                        "playerName": player.match_name,
                        "fromClub": buyer_name,
                        "fee": offer.fee,
                    })
                }))
            .collect::<Vec<_>>();
        let transfer_targets = self.game.players.iter()
            .filter(|player| player.transfer_listed && player.team_id.as_deref() != Some(team_id.as_str()) && !player.retired)
            .take(12)
            .map(|player| json!({
                "id": player.id,
                "name": player.match_name,
                "position": format!("{:?}", player.position),
                "marketValue": player.market_value,
            }))
            .collect::<Vec<_>>();
        let free_agents = self.game.players.iter()
            .filter(|player| player.team_id.is_none() && !player.retired)
            .take(12)
            .map(|player| json!({ "id": player.id, "name": player.match_name, "position": format!("{:?}", player.position) }))
            .collect::<Vec<_>>();
        Ok(json!({
            "currentDate": self.game.clock.current_date.format("%Y-%m-%d").to_string(),
            "club": {
                "id": team.id,
                "name": team.name,
                "finance": team.finance,
                "formation": team.formation,
                "playStyle": format!("{:?}", team.play_style),
            },
            "training": {
                "focus": format!("{:?}", team.training_focus),
                "intensity": format!("{:?}", team.training_intensity),
            },
            "nextFixture": next_fixture,
            "startingXiPlayerIds": team.starting_xi_ids,
            "squad": squad,
            "inbox": inbox,
            "incomingTransferOffers": incoming_transfer_offers,
            "transferTargets": transfer_targets,
            "freeAgents": free_agents,
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
            Command::SubmitTransferBid(body) => self.submit_transfer_bid(manager_id, body),
            Command::RespondTransferOffer(body) => self.respond_transfer_offer(manager_id, body),
            Command::SubmitContractOffer(body) => self.submit_contract_offer(manager_id, body),
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
        let selected_ids = body.player_ids.clone();
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
        Ok(json!({
            "teamId": team.id,
            "fixtureId": body.fixture_id,
            "playerIds": team.starting_xi_ids,
            "startingXiSet": true,
        }))
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

    fn submit_transfer_bid(
        &mut self,
        manager_id: Uuid,
        body: &SubmitTransferBidBody,
    ) -> Result<Value, ErrorCode> {
        if !body.installments_minor.is_empty() {
            return Err(ErrorCode::MatchCommandNotAllowed);
        }
        let fee = whole_currency(body.upfront_minor)?;
        let outcome = self.with_manager_context(manager_id, |game| {
            ofm_core::transfers::make_transfer_bid(game, &body.player_id, fee)
        })?;
        Ok(json!({
            "playerId": body.player_id,
            "decision": format!("{:?}", outcome.decision),
            "suggestedFee": outcome.suggested_fee,
            "isTerminal": outcome.is_terminal,
            "registrationDate": outcome.registration_date,
        }))
    }

    fn submit_contract_offer(
        &mut self,
        manager_id: Uuid,
        body: &SubmitContractOfferBody,
    ) -> Result<Value, ErrorCode> {
        let current_date = self.game.clock.current_date.date_naive();
        let years = i32::from(body.contract_end_year) - current_date.year();
        if years <= 0 || body.contract_end_month == 0 || body.contract_end_month > 12 {
            return Err(ErrorCode::MatchCommandNotAllowed);
        }
        let wage = whole_currency(body.weekly_wage_minor)?;
        let wage = u32::try_from(wage).map_err(|_| ErrorCode::InsufficientWageBudget)?;
        let outcome = self.with_manager_context(manager_id, |game| {
            ofm_core::contracts::offer_free_agent_contract(
                game,
                &body.player_id,
                ofm_core::contracts::RenewalOffer { weekly_wage: wage, contract_years: years as u32 },
            )
        })?;
        Ok(json!({
            "playerId": body.player_id,
            "decision": format!("{:?}", outcome.decision),
            "isTerminal": outcome.is_terminal,
            "suggestedWage": outcome.suggested_wage,
            "suggestedYears": outcome.suggested_years,
        }))
    }

    fn respond_transfer_offer(
        &mut self,
        manager_id: Uuid,
        body: &RespondTransferOfferBody,
    ) -> Result<Value, ErrorCode> {
        let team_id = self.controlled_team_id(manager_id)?;
        let player_id = self
            .game
            .players
            .iter()
            .find(|player| {
                player.team_id.as_deref() == Some(team_id.as_str())
                    && player.transfer_offers.iter().any(|offer| offer.id == body.offer_id)
            })
            .map(|player| player.id.clone())
            .ok_or(ErrorCode::AuthInvalid)?;
        match body.response {
            TransferOfferResponse::Accept => self.with_manager_context(manager_id, |game| {
                ofm_core::transfers::respond_to_offer(game, &player_id, &body.offer_id, true)
            })?,
            TransferOfferResponse::Reject => self.with_manager_context(manager_id, |game| {
                ofm_core::transfers::respond_to_offer(game, &player_id, &body.offer_id, false)
            })?,
            TransferOfferResponse::Counter => {
                let fee = whole_currency(body.counter_upfront_minor.ok_or(ErrorCode::InsufficientTransferBudget)?)?;
                self.with_manager_context(manager_id, |game| {
                    ofm_core::transfers::counter_offer(game, &player_id, &body.offer_id, fee)
                })?;
            }
        }
        Ok(json!({ "playerId": player_id, "offerId": body.offer_id, "response": format!("{:?}", body.response) }))
    }

    fn with_manager_context<T>(
        &mut self,
        manager_id: Uuid,
        operation: impl FnOnce(&mut Game) -> Result<T, String>,
    ) -> Result<T, ErrorCode> {
        let manager_id = manager_id.to_string();
        let acting_manager = self
            .game
            .managers
            .iter()
            .find(|manager| manager.id == manager_id)
            .cloned()
            .ok_or(ErrorCode::AuthInvalid)?;
        let previous_manager = std::mem::replace(&mut self.game.manager, acting_manager);
        let previous_manager_id = std::mem::replace(&mut self.game.manager_id, manager_id);
        let result = operation(&mut self.game).map_err(map_game_error);
        self.game.sync_user_manager_record();
        self.game.manager = previous_manager;
        self.game.manager_id = previous_manager_id;
        self.game.sync_user_manager_record();
        result
    }
}

fn whole_currency(minor: i64) -> Result<u64, ErrorCode> {
    if minor < 0 || minor % 100 != 0 {
        return Err(ErrorCode::InsufficientTransferBudget);
    }
    u64::try_from(minor / 100).map_err(|_| ErrorCode::InsufficientTransferBudget)
}

fn map_game_error(error: String) -> ErrorCode {
    if error.contains("transferWindow") {
        ErrorCode::TransferWindowClosed
    } else if error.contains("transferBudget") || error.contains("insufficientFunds") {
        ErrorCode::InsufficientTransferBudget
    } else if error.contains("boardWagePolicy") {
        ErrorCode::InsufficientWageBudget
    } else if error.contains("playerNotFound") {
        ErrorCode::AuthInvalid
    } else {
        ErrorCode::MatchCommandNotAllowed
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
    use domain::message::{InboxMessage, MessageCategory, MessageContext, MessagePriority};
    use domain::player::{Player, PlayerAttributes, Position, TransferOffer, TransferOfferStatus};
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
        let (mut career, manager_id) = career();
        let team_id = career.game.teams[0].id.clone();
        let mut competition = League { name: "Test League".into(), ..Default::default() };
        competition.fixtures.push(Fixture {
            id: Uuid::new_v4().to_string(), competition_id: "league".into(), matchday: 1,
            date: "2026-07-08".into(), home_team_id: team_id, away_team_id: Uuid::new_v4().to_string(),
            competition: FixtureCompetition::League, status: FixtureStatus::Scheduled, result: None,
        });
        career.game.competitions.push(competition);
        let view = career.manager_dashboard(manager_id).unwrap();
        assert_eq!(view["club"]["name"], "Albion");
        assert!(view["training"]["focus"].is_string());
        assert!(view["training"]["intensity"].is_string());
        assert_eq!(view["nextFixture"]["date"], "2026-07-08");
        assert_eq!(view["nextFixture"]["competition"], "Test League");
        assert!(view["startingXiPlayerIds"].is_array());
        assert!(view.get("players").is_none());
        assert!(view.get("managers").is_none());
    }

    #[test]
    fn manager_dashboard_inbox_excludes_another_clubs_private_message() {
        let (mut career, manager_id) = career();
        let own_team_id = career.game.teams[0].id.clone();
        let message = |id: &str, subject: &str, team_id: String| InboxMessage {
            id: id.into(), subject: subject.into(), body: "body".into(), sender: "Board".into(),
            sender_role: "Board".into(), date: "2026-07-01".into(), read: false,
            category: MessageCategory::System, priority: MessagePriority::Normal, actions: vec![],
            context: MessageContext { team_id: Some(team_id), ..Default::default() },
            subject_key: None, body_key: None, sender_key: None, sender_role_key: None,
            i18n_params: Default::default(),
        };
        career.game.messages.push(message("own", "Own message", own_team_id));
        career.game.messages.push(message("other", "Other message", Uuid::new_v4().to_string()));
        let inbox = career.manager_dashboard(manager_id).unwrap()["inbox"].as_array().unwrap().clone();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0]["subject"], "Own message");
    }

    #[test]
    fn manager_dashboard_exposes_only_public_transfer_target_fields() {
        let (mut career, manager_id) = career();
        let mut player = Player::new(
            "fpl-listed-1".into(), "Listed Player".into(), "Listed Player".into(), "1998-01-01".into(), "ENG".into(), Position::Forward,
            PlayerAttributes { pace: 60, stamina: 60, strength: 60, agility: 60, passing: 60, shooting: 60, tackling: 60, dribbling: 60, defending: 60, positioning: 60, vision: 60, decisions: 60, composure: 60, aggression: 60, teamwork: 60, leadership: 60, handling: 20, reflexes: 20, aerial: 60 },
        );
        player.transfer_listed = true;
        player.market_value = 1_500_000;
        career.game.players.push(player);
        let target = &career.manager_dashboard(manager_id).unwrap()["transferTargets"][0];
        assert_eq!(target["id"], "fpl-listed-1");
        assert_eq!(target["marketValue"], 1_500_000);
        assert!(target.get("attributes").is_none());
        assert!(target.get("ovr").is_none());
    }

    #[test]
    fn transfer_bids_reject_non_integral_minor_currency_before_mutating() {
        let (mut career, manager_id) = career();
        let before = career.game().teams[0].finance;
        let result = career.apply(manager_id, &Command::SubmitTransferBid(SubmitTransferBidBody {
            player_id: "fpl-unknown".into(),
            upfront_minor: 101,
            installments_minor: vec![],
        }));
        assert_eq!(result.unwrap_err(), ErrorCode::InsufficientTransferBudget);
        assert_eq!(career.game().teams[0].finance, before);
    }

    #[test]
    fn manager_can_reject_an_incoming_transfer_offer_canonically() {
        let (mut career, manager_id) = career();
        let buyer_id = Uuid::new_v4();
        career.game.teams.push(Team::new(
            buyer_id.to_string(), "Buyer".into(), "BUY".into(), "England".into(), "Buyer".into(), "Ground".into(), 20_000,
        ));
        let offer_id = "fpl-offer-1";
        let team_id = career.game.teams[0].id.clone();
        let mut player = Player::new(
            "fpl-99".into(), "Player".into(), "Test Player".into(), "1995-01-01".into(), "ENG".into(), Position::Midfielder,
            PlayerAttributes { pace: 60, stamina: 60, strength: 60, agility: 60, passing: 60, shooting: 60, tackling: 60, dribbling: 60, defending: 60, positioning: 60, vision: 60, decisions: 60, composure: 60, aggression: 60, teamwork: 60, leadership: 60, handling: 20, reflexes: 20, aerial: 60 },
        );
        player.team_id = Some(team_id);
        player.transfer_offers.push(TransferOffer {
            id: offer_id.into(), from_team_id: buyer_id.to_string(), fee: 1_000_000, wage_offered: 0,
            last_manager_fee: None, negotiation_round: 0, suggested_counter_fee: None,
            status: TransferOfferStatus::Pending, date: "2026-07-01".into(), registration_date: None,
        });
        career.game.players.push(player);
        let offers = career.manager_dashboard(manager_id).unwrap()["incomingTransferOffers"].as_array().unwrap().clone();
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0]["playerName"], "Player");

        career.apply(manager_id, &Command::RespondTransferOffer(RespondTransferOfferBody {
            offer_id: offer_id.into(), response: TransferOfferResponse::Reject, counter_upfront_minor: None,
        })).unwrap();
        assert_eq!(career.game.players[0].transfer_offers[0].status, TransferOfferStatus::Rejected);
    }
}
