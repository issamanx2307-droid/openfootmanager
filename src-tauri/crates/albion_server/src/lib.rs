//! The private, authoritative transport for a Project Albion career.
//!
//! This foundation owns session admission and the single command lane.  It is
//! intentionally independent of Tauri: desktop hosting and headless hosting
//! both use the same HTTP and WebSocket surface.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod canonical;
mod store;

use albion_protocol::command::{Command, MarkReadyBody};
use albion_protocol::event::{
    CommandAckBody, CommandRejectedBody, HelloBody, ReadyStateChangedBody, ServerEvent,
};
use albion_protocol::{
    CURRENT_VERSIONS, Envelope, EnvelopePayload, ErrorCode, MessageKind, ProtocolError,
    VersionSet,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;
use ofm_core::live_match_manager::LiveMatchSession;

pub use canonical::{Advancement, CanonicalCareer};
pub use store::{PersistedSessionState, SqliteCareerStore};

const MAX_MANAGER_SLOTS: usize = 2;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub career_id: Uuid,
    pub join_secret: String,
    pub versions: VersionSet,
    pub game: Option<ofm_core::game::Game>,
    store: Option<SqliteCareerStore>,
    persisted_session: Option<PersistedSessionState>,
}

impl ServerConfig {
    pub fn private_career(career_id: Uuid, join_secret: impl Into<String>) -> Self {
        Self {
            career_id,
            join_secret: join_secret.into(),
            versions: CURRENT_VERSIONS.to_owned_set(),
            game: None,
            store: None,
            persisted_session: None,
        }
    }

    pub fn open_save(path: impl AsRef<std::path::Path>, join_secret: impl Into<String>) -> Result<Self, String> {
        let (store, game, career_id) = SqliteCareerStore::open(path)?;
        let persisted_session = store.load_session_state()?;
        Ok(Self {
            career_id,
            join_secret: join_secret.into(),
            versions: CURRENT_VERSIONS.to_owned_set(),
            game: Some(game),
            store: Some(store),
            persisted_session,
        })
    }

    pub fn with_canonical_game(mut self, game: ofm_core::game::Game) -> Self {
        self.game = Some(game);
        self
    }
}

#[derive(Clone)]
pub struct AppState {
    session: Arc<Mutex<CareerSession>>,
    events: broadcast::Sender<ServerEvent>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        let (events, _) = broadcast::channel(128);
        Self { session: Arc::new(Mutex::new(CareerSession::new(config))), events }
    }
}

/// Creates the standalone server router.  Consumers can bind it with
/// `axum::serve`, or exercise it in-process in integration tests.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/version", get(version))
        .route("/api/v1/session/join", post(join))
        .route("/api/v1/session/reconnect", post(reconnect))
        .route("/ws", get(websocket))
        .with_state(state)
}

async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn readyz(State(state): State<AppState>) -> StatusCode {
    match state.session.lock() {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn version(State(state): State<AppState>) -> Json<VersionSet> {
    Json(state.session.lock().expect("career session lock poisoned").config.versions.clone())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRequest {
    pub join_secret: String,
    pub manager_id: Uuid,
    pub club_id: Uuid,
    pub client_versions: VersionSet,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinResponse {
    pub career_id: Uuid,
    pub manager_id: Uuid,
    pub reconnect_token: String,
    pub current_revision: u64,
    pub slot: ManagerSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagerSlot {
    Host,
    Guest,
}

async fn join(
    State(state): State<AppState>,
    Json(request): Json<JoinRequest>,
) -> Result<Json<JoinResponse>, ApiError> {
    let mut session = state.session.lock().expect("career session lock poisoned");
    let response = session.join(request)?;
    Ok(Json(response))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReconnectRequest {
    pub reconnect_token: String,
    pub client_versions: VersionSet,
}

async fn reconnect(
    State(state): State<AppState>,
    Json(request): Json<ReconnectRequest>,
) -> Result<Json<JoinResponse>, ApiError> {
    let session = state.session.lock().expect("career session lock poisoned");
    let response = session.reconnect(request)?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct WebSocketQuery {
    reconnect_token: String,
}

async fn websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WebSocketQuery>,
) -> Result<Response, ApiError> {
    let manager_id = {
        let session = state.session.lock().expect("career session lock poisoned");
        session.manager_for_token(&query.reconnect_token)?
    };
    Ok(ws.on_upgrade(move |socket| websocket_loop(socket, state, manager_id)))
}

async fn websocket_loop(mut socket: WebSocket, state: AppState, manager_id: Uuid) {
    {
        let mut session = state.session.lock().expect("career session lock poisoned");
        session.manager_connected(manager_id);
    }
    websocket_session(&mut socket, &state, manager_id).await;
    state
        .session
        .lock()
        .expect("career session lock poisoned")
        .manager_disconnected(manager_id);
}

async fn websocket_session(socket: &mut WebSocket, state: &AppState, manager_id: Uuid) {
    let initial_events = {
        let session = state.session.lock().expect("career session lock poisoned");
        let mut events = vec![ServerEvent::Hello(HelloBody {
                career_id: session.config.career_id,
                server_versions: session.config.versions.clone(),
                current_revision: session.revision,
            })];
        if let Some(view) = session.manager_dashboard_event(manager_id) {
            events.push(view);
        }
        events.extend(session.live_reconnect_events(manager_id));
        events
    };
    for event in initial_events {
        if send_event(socket, event).await.is_err() {
            return;
        }
    }

    let mut live_tick = tokio::time::interval(Duration::from_millis(500));
    let mut server_events = state.events.subscribe();
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else { return; };
                let Message::Text(text) = message else { continue; };
                let events = match serde_json::from_str::<Envelope>(&text) {
                    Ok(envelope) => state
                        .session
                        .lock()
                        .expect("career session lock poisoned")
                        .apply_command(manager_id, envelope),
                    Err(_) => vec![ServerEvent::CommandRejected(CommandRejectedBody {
                        command_id: Uuid::nil(),
                        current_revision: state.session.lock().expect("career session lock poisoned").revision,
                        error: ProtocolError::new(ErrorCode::ProtocolIncompatible),
                    })],
                };
                for event in events {
                    let _ = state.events.send(event);
                }
            }
            _ = live_tick.tick() => {
                let events = state.session.lock().expect("career session lock poisoned").tick_live_matches();
                for event in events {
                    let _ = state.events.send(event);
                }
            }
            event = server_events.recv() => match event {
                Ok(event) => {
                    if send_event(socket, event).await.is_err() { return; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }
}

async fn send_event(socket: &mut WebSocket, event: ServerEvent) -> Result<(), ()> {
    let text = serde_json::to_string(&event).map_err(|_| ())?;
    socket.send(Message::Text(text.into())).await.map_err(|_| ())
}

struct CareerSession {
    config: ServerConfig,
    revision: u64,
    slots: HashMap<Uuid, ManagerSlot>,
    claimed_clubs: HashMap<Uuid, Uuid>,
    reconnect_tokens: HashMap<String, Uuid>,
    manager_connection_counts: HashMap<Uuid, usize>,
    completed_commands: HashMap<Uuid, Vec<ServerEvent>>,
    ready_managers: HashSet<Uuid>,
    career: Option<CanonicalCareer>,
    store: Option<SqliteCareerStore>,
    live_matches: HashMap<Uuid, ActiveLiveMatch>,
    last_live_tick: Instant,
}

#[derive(Debug, Deserialize)]
struct PersistedClaims {
    slots: HashMap<Uuid, ManagerSlot>,
    claimed_clubs: HashMap<Uuid, Uuid>,
    #[serde(default)]
    live_matches: HashMap<Uuid, ActiveLiveMatch>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActiveLiveMatch {
    fixture_id: Uuid,
    session: LiveMatchSession,
    next_event_sequence: u64,
}

#[derive(Serialize)]
struct PersistedClaimsRef<'a> {
    slots: &'a HashMap<Uuid, ManagerSlot>,
    claimed_clubs: &'a HashMap<Uuid, Uuid>,
    live_matches: &'a HashMap<Uuid, ActiveLiveMatch>,
}

impl CareerSession {
    fn new(mut config: ServerConfig) -> Self {
        let career = config.game.take().map(CanonicalCareer::new);
        let store = config.store.take();
        let persisted_session = config.persisted_session.take();
        let mut restored_claims = persisted_session
            .as_ref()
            .and_then(|state| serde_json::from_str::<PersistedClaims>(&state.claims_json).ok());
        let reconnect_tokens = persisted_session
            .as_ref()
            .and_then(|state| serde_json::from_str::<HashMap<String, Uuid>>(&state.reconnect_tokens_json).ok())
            .unwrap_or_default();
        let mut live_matches = restored_claims
            .as_mut()
            .map_or_else(HashMap::new, |claims| std::mem::take(&mut claims.live_matches));
        for active in live_matches.values_mut() {
            active.session.reset_rng_after_restore();
        }
        Self {
            config,
            revision: persisted_session.as_ref().map_or(0, |state| state.career_revision),
            slots: restored_claims.as_ref().map_or_else(HashMap::new, |claims| claims.slots.clone()),
            claimed_clubs: restored_claims.map_or_else(HashMap::new, |claims| claims.claimed_clubs),
            reconnect_tokens,
            manager_connection_counts: HashMap::new(),
            completed_commands: HashMap::new(),
            ready_managers: HashSet::new(),
            career,
            store,
            live_matches,
            last_live_tick: Instant::now(),
        }
    }

    fn join(&mut self, request: JoinRequest) -> Result<JoinResponse, ApiError> {
        if request.join_secret != self.config.join_secret {
            return Err(ApiError::protocol(ErrorCode::AuthInvalid));
        }
        if !request.client_versions.is_compatible_with(&self.config.versions) {
            return Err(ApiError::protocol(ErrorCode::ProtocolIncompatible));
        }
        if self
            .career
            .as_ref()
            .is_some_and(|career| !career.manager_controls(request.manager_id, request.club_id))
        {
            return Err(ApiError::protocol(ErrorCode::AuthInvalid));
        }
        if let Some(manager) = self.claimed_clubs.get(&request.club_id) {
            if *manager != request.manager_id {
                return Err(ApiError::protocol(ErrorCode::ClubAlreadyControlled));
            }
        }
        if self
            .claimed_clubs
            .iter()
            .any(|(club_id, manager_id)| *manager_id == request.manager_id && *club_id != request.club_id)
        {
            return Err(ApiError::protocol(ErrorCode::ClubAlreadyControlled));
        }
        if !self.slots.contains_key(&request.manager_id) && self.slots.len() >= MAX_MANAGER_SLOTS {
            return Err(ApiError::protocol(ErrorCode::AuthInvalid));
        }

        let new_slot = if self.slots.is_empty() {
            ManagerSlot::Host
        } else {
            ManagerSlot::Guest
        };
        let slot = *self.slots.entry(request.manager_id).or_insert(new_slot);
        self.claimed_clubs.insert(request.club_id, request.manager_id);
        let reconnect_token = Uuid::new_v4().to_string();
        self.reconnect_tokens.insert(reconnect_token.clone(), request.manager_id);
        if self.persist_session_state().is_err() {
            return Err(ApiError::protocol(ErrorCode::SaveCorrupt));
        }
        Ok(JoinResponse {
            career_id: self.config.career_id,
            manager_id: request.manager_id,
            reconnect_token,
            current_revision: self.revision,
            slot,
        })
    }

    fn reconnect(&self, request: ReconnectRequest) -> Result<JoinResponse, ApiError> {
        if !request.client_versions.is_compatible_with(&self.config.versions) {
            return Err(ApiError::protocol(ErrorCode::ProtocolIncompatible));
        }
        let manager_id = self.manager_for_token(&request.reconnect_token)?;
        Ok(JoinResponse {
            career_id: self.config.career_id,
            manager_id,
            reconnect_token: request.reconnect_token,
            current_revision: self.revision,
            slot: *self.slots.get(&manager_id).expect("token manager has a slot"),
        })
    }

    fn manager_for_token(&self, token: &str) -> Result<Uuid, ApiError> {
        self.reconnect_tokens
            .get(token)
            .copied()
            .ok_or_else(|| ApiError::protocol(ErrorCode::AuthInvalid))
    }

    fn manager_connected(&mut self, manager_id: Uuid) {
        *self.manager_connection_counts.entry(manager_id).or_default() += 1;
    }

    fn manager_disconnected(&mut self, manager_id: Uuid) {
        let Some(count) = self.manager_connection_counts.get_mut(&manager_id) else { return; };
        *count = count.saturating_sub(1);
        if *count == 0 {
            self.manager_connection_counts.remove(&manager_id);
            self.ready_managers.remove(&manager_id);
        }
    }

    fn manager_is_connected(&self, manager_id: &Uuid) -> bool {
        self.manager_connection_counts.get(manager_id).is_some_and(|count| *count > 0)
    }

    fn persist_session_state(&self) -> Result<(), String> {
        let Some(store) = &self.store else { return Ok(()); };
        let claims = PersistedClaimsRef {
            slots: &self.slots,
            claimed_clubs: &self.claimed_clubs,
            live_matches: &self.live_matches,
        };
        store.checkpoint_session_state(&PersistedSessionState {
            career_revision: self.revision,
            claims_json: serde_json::to_string(&claims).map_err(|error| error.to_string())?,
            reconnect_tokens_json: serde_json::to_string(&self.reconnect_tokens)
                .map_err(|error| error.to_string())?,
        })
    }

    fn apply_command(
        &mut self,
        authenticated_manager_id: Uuid,
        envelope: Envelope,
    ) -> Vec<ServerEvent> {
        let command_id = envelope.command_id();
        if let Some(previous) = self.completed_commands.get(&command_id) {
            return previous.clone();
        }
        let mut event = self.apply_new_command(authenticated_manager_id, envelope);
        if event.iter().any(|item| matches!(item, ServerEvent::CommandAck(_)))
            && self.persist_session_state().is_err()
        {
            event = vec![ServerEvent::CommandRejected(CommandRejectedBody {
                command_id,
                current_revision: self.revision,
                error: ProtocolError::new(ErrorCode::SaveCorrupt),
            })];
        }
        self.completed_commands.insert(command_id, event.clone());
        event
    }

    fn apply_new_command(&mut self, manager_id: Uuid, envelope: Envelope) -> Vec<ServerEvent> {
        let command_id = envelope.command_id();
        let current_revision = self.revision;
        let reject = |code| {
            ServerEvent::CommandRejected(CommandRejectedBody {
                command_id,
                current_revision,
                error: ProtocolError::new(code),
            })
        };
        if envelope.protocol_version != self.config.versions.protocol_version {
            return vec![reject(ErrorCode::ProtocolIncompatible)];
        }
        if envelope.kind != MessageKind::Command
            || envelope.career_id != self.config.career_id
            || envelope.manager_id != manager_id
        {
            return vec![reject(ErrorCode::AuthInvalid)];
        }
        let EnvelopePayload::Command(command) = &envelope.payload else {
            return vec![reject(ErrorCode::ProtocolIncompatible)];
        };
        match command {
            Command::MarkReady(MarkReadyBody {}) => {
                self.ready_managers.insert(manager_id);
                self.revision += 1;
                let mut events = vec![ServerEvent::CommandAck(CommandAckBody {
                    command_id,
                    applied_revision: self.revision,
                }), self.ready_state_event(manager_id)];
                if self.ready_managers.len() == self.slots.len()
                    && self.slots.keys().all(|manager_id| self.manager_is_connected(manager_id))
                {
                    events.extend(self.advance_ready_barrier());
                }
                events
            }
            Command::ApplyLiveMatchCommand(body) => {
                let Some(expected_revision) = envelope.expected_revision else {
                    return vec![reject(ErrorCode::StaleRevision)];
                };
                if expected_revision != self.revision {
                    return vec![reject(ErrorCode::StaleRevision)];
                }
                let snapshot = match self.apply_live_command(manager_id, body) {
                    Ok(snapshot) => snapshot,
                    Err(error) => return vec![reject(error)],
                };
                self.revision += 1;
                vec![
                    ServerEvent::CommandAck(CommandAckBody { command_id, applied_revision: self.revision }),
                    ServerEvent::MatchState(albion_protocol::event::MatchStateBody {
                        match_id: body.match_id,
                        phase: format!("{:?}", snapshot.phase),
                        match_second: u32::from(snapshot.current_minute) * 60,
                        home_score: snapshot.home_score,
                        away_score: snapshot.away_score,
                    }),
                ]
            }
            command => {
                let Some(expected_revision) = envelope.expected_revision else {
                    return vec![reject(ErrorCode::StaleRevision)];
                };
                if expected_revision != self.revision {
                    return vec![reject(ErrorCode::StaleRevision)];
                }
                let Some(career) = self.career.as_mut() else {
                    return vec![reject(ErrorCode::MatchCommandNotAllowed)];
                };
                let previous = career.clone();
                let changes = match career.apply(manager_id, command) {
                    Ok(changes) => changes,
                    Err(error) => return vec![reject(error)],
                };
                if self
                    .store
                    .as_ref()
                    .is_some_and(|store| store.checkpoint(career.game()).is_err())
                {
                    *career = previous;
                    return vec![reject(ErrorCode::SaveCorrupt)];
                }
                let from_revision = self.revision;
                self.revision += 1;
                vec![
                    ServerEvent::CommandAck(CommandAckBody { command_id, applied_revision: self.revision }),
                    ServerEvent::StateDelta(albion_protocol::event::StateDeltaBody {
                        from_revision,
                        to_revision: self.revision,
                        changes,
                    }),
                ]
            }
        }
    }

    fn ready_state_event(&self, manager_id: Uuid) -> ServerEvent {
        let other_manager_ready = self
            .slots
            .keys()
            .any(|other_id| *other_id != manager_id && self.ready_managers.contains(other_id));
        ServerEvent::ReadyStateChanged(ReadyStateChangedBody {
            manager_id,
            is_ready: self.ready_managers.contains(&manager_id),
            other_manager_ready,
        })
    }

    fn manager_dashboard_event(&self, manager_id: Uuid) -> Option<ServerEvent> {
        let career = self.career.as_ref()?;
        let payload = career.manager_dashboard(manager_id).ok()?;
        Some(ServerEvent::ViewSnapshot(albion_protocol::event::ViewSnapshotBody {
            revision: self.revision,
            view_name: "dashboard".into(),
            payload,
        }))
    }

    fn live_reconnect_events(&self, manager_id: Uuid) -> Vec<ServerEvent> {
        self.live_matches.iter().filter_map(|(match_id, active)| {
            let home_club_id = Uuid::parse_str(&active.session.home_team_id).ok()?;
            let away_club_id = Uuid::parse_str(&active.session.away_team_id).ok()?;
            let controls_match = [home_club_id, away_club_id]
                .into_iter()
                .any(|club_id| self.claimed_clubs.get(&club_id) == Some(&manager_id));
            if !controls_match {
                return None;
            }
            let snapshot = active.session.snapshot();
            Some(vec![
                ServerEvent::MatchOpened(albion_protocol::event::MatchOpenedBody {
                    match_id: *match_id,
                    fixture_id: active.fixture_id,
                    home_club_id,
                    away_club_id,
                }),
                ServerEvent::MatchState(albion_protocol::event::MatchStateBody {
                    match_id: *match_id,
                    phase: format!("{:?}", snapshot.phase),
                    match_second: u32::from(snapshot.current_minute) * 60,
                    home_score: snapshot.home_score,
                    away_score: snapshot.away_score,
                }),
            ])
        }).flatten().collect()
    }

    fn apply_live_command(
        &mut self,
        manager_id: Uuid,
        body: &albion_protocol::command::ApplyLiveMatchCommandBody,
    ) -> Result<engine::MatchSnapshot, ErrorCode> {
        let club_id = self
            .career
            .as_ref()
            .ok_or(ErrorCode::MatchCommandNotAllowed)?
            .controlled_club(manager_id)?;
        let session = self
            .live_matches
            .get_mut(&body.match_id)
            .ok_or(ErrorCode::MatchCommandNotAllowed)?;
        let side = if session.session.home_team_id == club_id.to_string() {
            engine::Side::Home
        } else if session.session.away_team_id == club_id.to_string() {
            engine::Side::Away
        } else {
            return Err(ErrorCode::AuthInvalid);
        };
        let command = match &body.command {
            albion_protocol::command::LiveMatchCommandKind::Substitute { player_out_id, player_in_id } => {
                engine::MatchCommand::Substitute {
                    side,
                    player_off_id: player_out_id.to_string(),
                    player_on_id: player_in_id.to_string(),
                }
            }
            albion_protocol::command::LiveMatchCommandKind::ChangeFormation { formation } => {
                engine::MatchCommand::ChangeFormation { side, formation: formation.clone() }
            }
            albion_protocol::command::LiveMatchCommandKind::SetTeamInstruction { key, value }
                if key == "play_style" => {
                    let play_style = match value.as_str() {
                        "balanced" => engine::PlayStyle::Balanced,
                        "attacking" => engine::PlayStyle::Attacking,
                        "defensive" => engine::PlayStyle::Defensive,
                        "possession" => engine::PlayStyle::Possession,
                        "counter" => engine::PlayStyle::Counter,
                        "high_press" | "highpress" => engine::PlayStyle::HighPress,
                        _ => return Err(ErrorCode::MatchCommandNotAllowed),
                    };
                    engine::MatchCommand::ChangePlayStyle { side, play_style }
                }
            _ => return Err(ErrorCode::MatchCommandNotAllowed),
        };
        session.session.apply_command(command).map_err(|_| ErrorCode::MatchCommandNotAllowed)?;
        Ok(session.session.snapshot())
    }

    fn advance_ready_barrier(&mut self) -> Vec<ServerEvent> {
        let controlled_clubs = self.claimed_clubs.keys().copied().collect();
        let Some(career) = self.career.as_mut() else {
            self.ready_managers.clear();
            return Vec::new();
        };
        let previous = career.clone();
        let outcome = career.advance_until_human_blocker(&controlled_clubs);
        if self
            .store
            .as_ref()
            .is_some_and(|store| store.checkpoint(career.game()).is_err())
        {
            *career = previous;
            self.ready_managers.clear();
            return vec![ServerEvent::ServerNotice(albion_protocol::event::ServerNoticeBody {
                severity: albion_protocol::event::ServerNoticeSeverity::Critical,
                message_key: "server.saveFailed".into(),
            })];
        }
        self.ready_managers.clear();
        self.revision += 1;
        match outcome {
            Advancement::AdvancedThrough { date: _ } => vec![ServerEvent::GameTimeChanged(
                albion_protocol::event::GameTimeChangedBody {
                    career_year: career.game().clock.current_date.format("%Y").to_string().parse().unwrap_or_default(),
                    career_month: career.game().clock.current_date.format("%-m").to_string().parse().unwrap_or_default(),
                    career_day: career.game().clock.current_date.format("%-d").to_string().parse().unwrap_or_default(),
                    revision: self.revision,
                },
            ), ServerEvent::ServerNotice(albion_protocol::event::ServerNoticeBody {
                severity: albion_protocol::event::ServerNoticeSeverity::Info,
                message_key: "server.advancedThrough".into(),
            })],
            Advancement::HumanFixture { fixture_id, home_club_id, away_club_id } => {
                let match_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, fixture_id.as_bytes());
                match career.open_live_match(fixture_id, &controlled_clubs) {
                    Ok(session) => {
                        self.live_matches.insert(match_id, ActiveLiveMatch {
                            fixture_id,
                            session,
                            next_event_sequence: 1,
                        });
                        vec![ServerEvent::MatchOpened(albion_protocol::event::MatchOpenedBody {
                            match_id,
                            fixture_id,
                            home_club_id,
                            away_club_id,
                        })]
                    }
                    Err(_) => vec![ServerEvent::ServerNotice(albion_protocol::event::ServerNoticeBody {
                        severity: albion_protocol::event::ServerNoticeSeverity::Critical,
                        message_key: "server.liveMatchOpenFailed".into(),
                    })],
                }
            }
        }
    }

    fn tick_live_matches(&mut self) -> Vec<ServerEvent> {
        if self.live_matches.is_empty() || self.last_live_tick.elapsed() < Duration::from_millis(500) {
            return Vec::new();
        }
        self.last_live_tick = Instant::now();
        let match_ids: Vec<Uuid> = self.live_matches.keys().copied().collect();
        let mut events = Vec::new();
        for match_id in match_ids {
            let Some(mut active) = self.live_matches.remove(&match_id) else { continue; };
            if self.has_disconnected_human_for_match(&active.session) {
                self.live_matches.insert(match_id, active);
                continue;
            }
            let minute = active.session.step();
            let snapshot = active.session.snapshot();
            let event_count = minute.events.len() as u64;
            if event_count > 0 {
                let from_seq = active.next_event_sequence;
                active.next_event_sequence += event_count;
                events.push(ServerEvent::MatchEventBatch(albion_protocol::event::MatchEventBatchBody {
                    match_id,
                    from_seq,
                    to_seq: active.next_event_sequence - 1,
                    events: minute.events.iter().map(|event| serde_json::to_value(event).unwrap_or(serde_json::Value::Null)).collect(),
                }));
            }
            events.push(ServerEvent::MatchState(albion_protocol::event::MatchStateBody {
                match_id,
                phase: format!("{:?}", snapshot.phase),
                match_second: u32::from(snapshot.current_minute) * 60,
                home_score: snapshot.home_score,
                away_score: snapshot.away_score,
            }));
            if minute.is_finished {
                let Some(career) = self.career.as_mut() else { continue; };
                let previous = career.clone();
                match career.finish_live_match(active.session) {
                    Ok(report) if !self.store.as_ref().is_some_and(|store| store.checkpoint(career.game()).is_err()) => {
                        self.revision += 1;
                        events.push(ServerEvent::MatchFinished(albion_protocol::event::MatchFinishedBody {
                            match_id,
                            home_score: snapshot.home_score,
                            away_score: snapshot.away_score,
                            report,
                        }));
                    }
                    _ => {
                        *career = previous;
                        events.push(ServerEvent::ServerNotice(albion_protocol::event::ServerNoticeBody {
                            severity: albion_protocol::event::ServerNoticeSeverity::Critical,
                            message_key: "server.liveMatchSaveFailed".into(),
                        }));
                    }
                }
            } else {
                self.live_matches.insert(match_id, active);
            }
        }
        if !events.is_empty() && self.persist_session_state().is_err() {
            events.push(ServerEvent::ServerNotice(albion_protocol::event::ServerNoticeBody {
                severity: albion_protocol::event::ServerNoticeSeverity::Critical,
                message_key: "server.liveMatchSaveFailed".into(),
            }));
        }
        events
    }

    fn has_disconnected_human_for_match(&self, live_match: &LiveMatchSession) -> bool {
        [live_match.home_team_id.as_str(), live_match.away_team_id.as_str()]
            .into_iter()
            .filter_map(|club_id| Uuid::parse_str(club_id).ok())
            .filter_map(|club_id| self.claimed_clubs.get(&club_id))
            .any(|manager_id| !self.manager_is_connected(manager_id))
    }
}

#[derive(Debug)]
struct ApiError(ProtocolError);

impl ApiError {
    fn protocol(code: ErrorCode) -> Self {
        Self(ProtocolError::new(code))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code {
            ErrorCode::AuthInvalid => StatusCode::UNAUTHORIZED,
            ErrorCode::ProtocolIncompatible => StatusCode::UPGRADE_REQUIRED,
            ErrorCode::ClubAlreadyControlled => StatusCode::CONFLICT,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, Json(self.0)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use albion_protocol::command::SetTacticsBody;
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::Request;
    use axum::http::header::CONTENT_TYPE;
    use chrono::{TimeZone, Utc};
    use db::game_database::GameDatabase;
    use db::game_persistence::GamePersistenceWriter;
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use futures_util::{SinkExt, StreamExt};
    use std::future::IntoFuture;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use tower::ServiceExt;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
    use tempfile::tempdir;

    fn session() -> CareerSession {
        CareerSession::new(ServerConfig::private_career(Uuid::new_v4(), "private-secret"))
    }

    fn join_request(manager_id: Uuid, club_id: Uuid) -> JoinRequest {
        JoinRequest {
            join_secret: "private-secret".into(),
            manager_id,
            club_id,
            client_versions: CURRENT_VERSIONS.to_owned_set(),
        }
    }

    fn ready_envelope(session: &CareerSession, manager_id: Uuid, command_id: Uuid) -> Envelope {
        Envelope {
            protocol_version: session.config.versions.protocol_version,
            message_id: command_id,
            kind: MessageKind::Command,
            career_id: session.config.career_id,
            manager_id,
            expected_revision: None,
            payload: EnvelopePayload::Command(Command::MarkReady(MarkReadyBody {})),
        }
    }

    fn two_manager_game() -> (Game, Uuid, Uuid, Uuid, Uuid) {
        let manager_a_id = Uuid::new_v4();
        let manager_b_id = Uuid::new_v4();
        let club_a_id = Uuid::new_v4();
        let club_b_id = Uuid::new_v4();
        let mut manager_a = Manager::new(manager_a_id.to_string(), "Alex".into(), "Host".into(), "1980-01-01".into(), "ENG".into());
        manager_a.hire(club_a_id.to_string());
        let mut manager_b = Manager::new(manager_b_id.to_string(), "Bea".into(), "Guest".into(), "1981-01-01".into(), "ENG".into());
        manager_b.hire(club_b_id.to_string());
        let mut club_a = Team::new(club_a_id.to_string(), "Alpha".into(), "ALP".into(), "England".into(), "Alpha".into(), "Ground A".into(), 20_000);
        club_a.manager_id = Some(manager_a_id.to_string());
        let mut club_b = Team::new(club_b_id.to_string(), "Beta".into(), "BET".into(), "England".into(), "Beta".into(), "Ground B".into(), 20_000);
        club_b.manager_id = Some(manager_b_id.to_string());
        let mut game = Game::new(
            GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap()),
            manager_a,
            vec![club_a, club_b],
            vec![],
            vec![],
            vec![],
        );
        game.managers.push(manager_b);
        (game, manager_a_id, manager_b_id, club_a_id, club_b_id)
    }

    fn live_player(id: String, team_id: &Uuid, position: Position) -> Player {
        let mut player = Player::new(id, "Player".into(), "Live Player".into(), "1995-01-01".into(), "ENG".into(), position, PlayerAttributes {
            pace: 65, stamina: 65, strength: 65, agility: 65, passing: 65, shooting: 65,
            tackling: 65, dribbling: 65, defending: 65, positioning: 65, vision: 65,
            decisions: 65, composure: 65, aggression: 50, teamwork: 65, leadership: 50,
            handling: 65, reflexes: 65, aerial: 65,
        });
        player.team_id = Some(team_id.to_string());
        player
    }

    fn two_manager_live_game() -> (Game, Uuid, Uuid, Uuid, Uuid) {
        let (mut game, manager_a, manager_b, club_a, club_b) = two_manager_game();
        let positions = [
            Position::Goalkeeper, Position::Defender, Position::Defender, Position::Defender,
            Position::Defender, Position::Midfielder, Position::Midfielder, Position::Midfielder,
            Position::Midfielder, Position::Forward, Position::Forward,
        ];
        game.players.extend(positions.iter().enumerate().flat_map(|(index, position)| [
            live_player(format!("a-{index}"), &club_a, position.clone()),
            live_player(format!("b-{index}"), &club_b, position.clone()),
        ]));
        let fixture_id = Uuid::new_v4();
        let mut league = League::default();
        league.id = "test-league".into();
        league.fixtures.push(Fixture {
            id: fixture_id.to_string(),
            competition_id: league.id.clone(),
            matchday: 1,
            date: game.clock.current_date.format("%Y-%m-%d").to_string(),
            home_team_id: club_a.to_string(),
            away_team_id: club_b.to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        });
        league.standings = vec![StandingEntry::new(club_a.to_string()), StandingEntry::new(club_b.to_string())];
        game.competitions.push(league);
        (game, manager_a, manager_b, club_a, club_b)
    }

    fn tactics_envelope(session: &CareerSession, manager_id: Uuid, expected_revision: u64) -> Envelope {
        Envelope {
            protocol_version: session.config.versions.protocol_version,
            message_id: Uuid::new_v4(),
            kind: MessageKind::Command,
            career_id: session.config.career_id,
            manager_id,
            expected_revision: Some(expected_revision),
            payload: EnvelopePayload::Command(Command::SetTactics(SetTacticsBody {
                formation: "4-3-3".into(),
                mentality: "high_press".into(),
            })),
        }
    }

    #[test]
    fn join_requires_secret_and_different_clubs() {
        let mut session = session();
        let manager_a = Uuid::new_v4();
        let club = Uuid::new_v4();
        session.join(join_request(manager_a, club)).unwrap();
        let error = session.join(join_request(Uuid::new_v4(), club)).unwrap_err();
        assert_eq!(error.0.code, ErrorCode::ClubAlreadyControlled);
    }

    #[test]
    fn a_manager_cannot_claim_a_second_club_through_join() {
        let mut session = session();
        let manager = Uuid::new_v4();
        session.join(join_request(manager, Uuid::new_v4())).unwrap();
        let error = session.join(join_request(manager, Uuid::new_v4())).unwrap_err();
        assert_eq!(error.0.code, ErrorCode::ClubAlreadyControlled);
    }

    #[test]
    fn duplicate_ready_is_idempotent() {
        let mut session = session();
        let manager = Uuid::new_v4();
        session.join(join_request(manager, Uuid::new_v4())).unwrap();
        let command_id = Uuid::new_v4();
        let first = session.apply_command(manager, ready_envelope(&session, manager, command_id));
        let second = session.apply_command(manager, ready_envelope(&session, manager, command_id));
        let ServerEvent::CommandAck(first) = &first[0] else { panic!("ready should acknowledge") };
        let ServerEvent::CommandAck(second) = &second[0] else { panic!("retry should acknowledge") };
        assert_eq!(first.applied_revision, 1);
        assert_eq!(second.applied_revision, 1);
        assert!(matches!(session.apply_command(manager, ready_envelope(&session, manager, command_id))[1], ServerEvent::ReadyStateChanged(_)));
        assert_eq!(session.revision, 1);
    }

    #[test]
    fn disconnect_clears_ready_and_blocks_the_ready_barrier() {
        let mut session = session();
        let manager = Uuid::new_v4();
        session.join(join_request(manager, Uuid::new_v4())).unwrap();
        session.join(join_request(Uuid::new_v4(), Uuid::new_v4())).unwrap();
        session.manager_connected(manager);
        session.apply_command(manager, ready_envelope(&session, manager, Uuid::new_v4()));
        assert!(session.ready_managers.contains(&manager));

        session.manager_disconnected(manager);
        assert!(!session.ready_managers.contains(&manager));
        assert!(!session.manager_is_connected(&manager));
    }

    #[test]
    fn closing_one_of_a_managers_tabs_keeps_the_manager_connected() {
        let mut session = session();
        let manager = Uuid::new_v4();
        session.manager_connected(manager);
        session.manager_connected(manager);
        session.manager_disconnected(manager);
        assert!(session.manager_is_connected(&manager));
        session.manager_disconnected(manager);
        assert!(!session.manager_is_connected(&manager));
    }

    #[test]
    fn two_ready_humans_open_and_tick_one_canonical_live_match() {
        let (game, manager_a, manager_b, club_a, club_b) = two_manager_live_game();
        let mut session = CareerSession::new(
            ServerConfig::private_career(Uuid::new_v4(), "private-secret").with_canonical_game(game),
        );
        session.join(join_request(manager_a, club_a)).unwrap();
        session.join(join_request(manager_b, club_b)).unwrap();
        session.manager_connected(manager_a);
        session.manager_connected(manager_b);
        session.apply_command(manager_a, ready_envelope(&session, manager_a, Uuid::new_v4()));
        let opened = session.apply_command(manager_b, ready_envelope(&session, manager_b, Uuid::new_v4()));
        assert!(opened.iter().any(|event| matches!(event, ServerEvent::MatchOpened(_))));
        assert_eq!(session.live_matches.len(), 1);

        session.last_live_tick = Instant::now() - Duration::from_millis(501);
        let updates = session.tick_live_matches();
        assert!(updates.iter().any(|event| matches!(event, ServerEvent::MatchState(_))));
    }

    #[test]
    fn one_human_can_open_and_tick_a_canonical_match_against_ai() {
        let (mut game, manager_a, manager_b, club_a, club_b) = two_manager_live_game();
        game.managers.retain(|manager| manager.id == manager_a.to_string());
        game.teams
            .iter_mut()
            .find(|team| team.id == club_b.to_string())
            .unwrap()
            .manager_id = None;
        assert_ne!(manager_a, manager_b);

        let mut session = CareerSession::new(
            ServerConfig::private_career(Uuid::new_v4(), "private-secret").with_canonical_game(game),
        );
        session.join(join_request(manager_a, club_a)).unwrap();
        session.manager_connected(manager_a);
        let opened = session.apply_command(manager_a, ready_envelope(&session, manager_a, Uuid::new_v4()));
        assert!(opened.iter().any(|event| matches!(event, ServerEvent::MatchOpened(_))));
        assert_eq!(session.live_matches.len(), 1);

        session.last_live_tick = Instant::now() - Duration::from_millis(501);
        let updates = session.tick_live_matches();
        assert!(updates.iter().any(|event| matches!(event, ServerEvent::MatchState(_))));
    }

    #[test]
    fn human_live_match_pauses_until_the_disconnected_manager_returns() {
        let (game, manager_a, manager_b, club_a, club_b) = two_manager_live_game();
        let mut session = CareerSession::new(
            ServerConfig::private_career(Uuid::new_v4(), "private-secret").with_canonical_game(game),
        );
        session.join(join_request(manager_a, club_a)).unwrap();
        session.join(join_request(manager_b, club_b)).unwrap();
        session.manager_connected(manager_a);
        session.manager_connected(manager_b);
        session.apply_command(manager_a, ready_envelope(&session, manager_a, Uuid::new_v4()));
        session.apply_command(manager_b, ready_envelope(&session, manager_b, Uuid::new_v4()));
        assert_eq!(session.live_matches.len(), 1);

        session.manager_disconnected(manager_b);
        session.last_live_tick = Instant::now() - Duration::from_millis(501);
        assert!(session.tick_live_matches().is_empty());

        session.manager_connected(manager_b);
        session.last_live_tick = Instant::now() - Duration::from_millis(501);
        assert!(session.tick_live_matches().iter().any(|event| matches!(event, ServerEvent::MatchState(_))));
    }

    #[test]
    fn restart_restores_claim_and_reconnect_token_from_sqlite() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("albion.db");
        let (game, manager_a, _, club_a, _) = two_manager_game();
        let db = GameDatabase::open(&path).unwrap();
        GamePersistenceWriter::write_game(&db, &game, "albion-save", "Albion").unwrap();
        drop(db);

        let config = ServerConfig::open_save(&path, "private-secret").unwrap();
        let mut original = CareerSession::new(config);
        let joined = original.join(join_request(manager_a, club_a)).unwrap();
        let command = tactics_envelope(&original, manager_a, 0);
        assert!(matches!(original.apply_command(manager_a, command)[0], ServerEvent::CommandAck(_)));
        drop(original);

        let restarted = CareerSession::new(ServerConfig::open_save(&path, "private-secret").unwrap());
        let reconnected = restarted.reconnect(ReconnectRequest {
            reconnect_token: joined.reconnect_token.clone(),
            client_versions: CURRENT_VERSIONS.to_owned_set(),
        }).unwrap();
        assert_eq!(reconnected.manager_id, manager_a);
        assert_eq!(reconnected.slot, ManagerSlot::Host);
        assert_eq!(reconnected.reconnect_token, joined.reconnect_token);
        assert_eq!(reconnected.current_revision, 1);
        assert_eq!(restarted.claimed_clubs.get(&club_a), Some(&manager_a));
    }

    #[test]
    fn restart_restores_an_in_progress_live_match_from_sqlite() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("albion-live.db");
        let (game, manager_a, manager_b, club_a, club_b) = two_manager_live_game();
        let db = GameDatabase::open(&path).unwrap();
        GamePersistenceWriter::write_game(&db, &game, "albion-live", "Albion Live").unwrap();
        drop(db);

        let mut original = CareerSession::new(ServerConfig::open_save(&path, "private-secret").unwrap());
        original.join(join_request(manager_a, club_a)).unwrap();
        original.join(join_request(manager_b, club_b)).unwrap();
        original.manager_connected(manager_a);
        original.manager_connected(manager_b);
        original.apply_command(manager_a, ready_envelope(&original, manager_a, Uuid::new_v4()));
        original.apply_command(manager_b, ready_envelope(&original, manager_b, Uuid::new_v4()));
        let match_id = *original.live_matches.keys().next().unwrap();
        original.last_live_tick = Instant::now() - Duration::from_millis(501);
        original.tick_live_matches();
        let before = original.live_matches.get(&match_id).unwrap().session.snapshot();
        drop(original);

        let restarted = CareerSession::new(ServerConfig::open_save(&path, "private-secret").unwrap());
        let restored = restarted.live_matches.get(&match_id).expect("live match checkpoint").session.snapshot();
        assert_eq!(restored.current_minute, before.current_minute);
        assert_eq!(restored.home_score, before.home_score);
        assert_eq!(restored.away_score, before.away_score);
        let replay = restarted.live_reconnect_events(manager_a);
        assert!(replay.iter().any(|event| matches!(event, ServerEvent::MatchOpened(body) if body.match_id == match_id)));
        assert!(replay.iter().any(|event| matches!(event, ServerEvent::MatchState(body)
            if body.match_id == match_id && body.match_second == u32::from(before.current_minute) * 60)));
    }

    #[tokio::test]
    async fn health_and_readiness_endpoints_are_available() {
        let state = AppState::new(ServerConfig::private_career(Uuid::new_v4(), "private-secret"));
        for path in ["/healthz", "/readyz"] {
            let response = router(state.clone())
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NO_CONTENT, "{path}");
        }
    }

    #[tokio::test]
    async fn http_join_assigns_two_distinct_clubs_and_rejects_a_duplicate_claim() {
        let state = AppState::new(ServerConfig::private_career(Uuid::new_v4(), "private-secret"));
        let club_a = Uuid::new_v4();
        let club_b = Uuid::new_v4();
        let manager_a = Uuid::new_v4();
        let manager_b = Uuid::new_v4();
        for request in [join_request(manager_a, club_a), join_request(manager_b, club_b)] {
            let response = router(state.clone())
                .oneshot(Request::post("/api/v1/session/join")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let joined: JoinResponse = serde_json::from_slice(&body).unwrap();
            assert!(matches!(joined.slot, ManagerSlot::Host | ManagerSlot::Guest));
        }
        let duplicate = join_request(Uuid::new_v4(), club_a);
        let response = router(state)
            .oneshot(Request::post("/api/v1/session/join")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&duplicate).unwrap()))
                .unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn reconnect_restores_the_original_manager_slot() {
        let state = AppState::new(ServerConfig::private_career(Uuid::new_v4(), "private-secret"));
        let manager_id = Uuid::new_v4();
        let join = join_request(manager_id, Uuid::new_v4());
        let response = router(state.clone())
            .oneshot(Request::post("/api/v1/session/join")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&join).unwrap()))
                .unwrap())
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let joined: JoinResponse = serde_json::from_slice(&body).unwrap();
        let reconnect = ReconnectRequest {
            reconnect_token: joined.reconnect_token.clone(),
            client_versions: CURRENT_VERSIONS.to_owned_set(),
        };
        let response = router(state)
            .oneshot(Request::post("/api/v1/session/reconnect")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&reconnect).unwrap()))
                .unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let restored: JoinResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(restored.manager_id, manager_id);
        assert_eq!(restored.slot, joined.slot);
        assert_eq!(restored.reconnect_token, joined.reconnect_token);
    }

    #[tokio::test]
    async fn two_websocket_clients_receive_hello_and_duplicate_ready_is_safe() {
        let state = AppState::new(ServerConfig::private_career(Uuid::new_v4(), "private-secret"));
        let (token_a, token_b, manager_a) = {
            let mut session = state.session.lock().unwrap();
            let manager_a = Uuid::new_v4();
            let joined_a = session.join(join_request(manager_a, Uuid::new_v4())).unwrap();
            let joined_b = session.join(join_request(Uuid::new_v4(), Uuid::new_v4())).unwrap();
            (joined_a.reconnect_token, joined_b.reconnect_token, manager_a)
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(axum::serve(listener, router(state.clone())).into_future());
        let (mut client_a, _) = connect_async(format!("ws://{address}/ws?reconnect_token={token_a}")).await.unwrap();
        let (mut client_b, _) = connect_async(format!("ws://{address}/ws?reconnect_token={token_b}")).await.unwrap();

        for client in [&mut client_a, &mut client_b] {
            let Some(Ok(TungsteniteMessage::Text(text))) = client.next().await else { panic!("server must send hello") };
            assert!(matches!(serde_json::from_str::<ServerEvent>(&text).unwrap(), ServerEvent::Hello(_)));
        }
        let (career_id, protocol_version) = {
            let session = state.session.lock().unwrap();
            (session.config.career_id, session.config.versions.protocol_version)
        };
        let ready = Envelope {
            protocol_version,
            message_id: Uuid::new_v4(),
            kind: MessageKind::Command,
            career_id,
            manager_id: manager_a,
            expected_revision: None,
            payload: EnvelopePayload::Command(Command::MarkReady(MarkReadyBody {})),
        };
        let message = serde_json::to_string(&ready).unwrap();
        client_a.send(TungsteniteMessage::Text(message.clone().into())).await.unwrap();
        let Some(Ok(TungsteniteMessage::Text(text))) = client_a.next().await else { panic!("ready needs acknowledgement") };
        assert!(matches!(serde_json::from_str::<ServerEvent>(&text).unwrap(), ServerEvent::CommandAck(_)));
        let Some(Ok(TungsteniteMessage::Text(_))) = client_a.next().await else { panic!("ready state event must follow acknowledgement") };
        let Some(Ok(TungsteniteMessage::Text(text))) = client_b.next().await else { panic!("guest must receive the shared acknowledgement") };
        assert!(matches!(serde_json::from_str::<ServerEvent>(&text).unwrap(), ServerEvent::CommandAck(_)));
        let Some(Ok(TungsteniteMessage::Text(text))) = client_b.next().await else { panic!("guest must receive the shared ready state") };
        assert!(matches!(serde_json::from_str::<ServerEvent>(&text).unwrap(), ServerEvent::ReadyStateChanged(_)));
        client_a.send(TungsteniteMessage::Text(message.into())).await.unwrap();
        let Some(Ok(TungsteniteMessage::Text(text))) = client_a.next().await else { panic!("duplicate needs acknowledgement") };
        let ServerEvent::CommandAck(ack) = serde_json::from_str::<ServerEvent>(&text).unwrap() else { panic!("duplicate must replay ack") };
        assert_eq!(ack.applied_revision, 1);
        server.abort();
    }

    #[test]
    fn conflicting_two_manager_commands_commit_once_at_one_revision() {
        let (game, manager_a, manager_b, club_a, club_b) = two_manager_game();
        let mut session = CareerSession::new(
            ServerConfig::private_career(Uuid::new_v4(), "private-secret").with_canonical_game(game),
        );
        session.join(join_request(manager_a, club_a)).unwrap();
        session.join(join_request(manager_b, club_b)).unwrap();

        let first = session.apply_command(manager_a, tactics_envelope(&session, manager_a, 0));
        let second = session.apply_command(manager_b, tactics_envelope(&session, manager_b, 0));

        assert!(matches!(first[0], ServerEvent::CommandAck(CommandAckBody { applied_revision: 1, .. })));
        assert!(matches!(second[0], ServerEvent::CommandRejected(CommandRejectedBody { error: ProtocolError { code: ErrorCode::StaleRevision, .. }, .. })));
        assert_eq!(session.revision, 1);
        assert_eq!(session.career.as_ref().unwrap().game().teams[0].formation, "4-3-3");
        assert_eq!(session.career.as_ref().unwrap().game().teams[1].formation, "4-4-2");
    }
}
