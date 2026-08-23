//! The private, authoritative transport for a Project Albion career.
//!
//! This foundation owns session admission and the single command lane.  It is
//! intentionally independent of Tauri: desktop hosting and headless hosting
//! both use the same HTTP and WebSocket surface.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

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
use uuid::Uuid;

const MAX_MANAGER_SLOTS: usize = 2;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub career_id: Uuid,
    pub join_secret: String,
    pub versions: VersionSet,
}

impl ServerConfig {
    pub fn private_career(career_id: Uuid, join_secret: impl Into<String>) -> Self {
        Self {
            career_id,
            join_secret: join_secret.into(),
            versions: CURRENT_VERSIONS.to_owned_set(),
        }
    }
}

#[derive(Clone)]
pub struct AppState(Arc<Mutex<CareerSession>>);

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self(Arc::new(Mutex::new(CareerSession::new(config))))
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
    match state.0.lock() {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn version(State(state): State<AppState>) -> Json<VersionSet> {
    Json(state.0.lock().expect("career session lock poisoned").config.versions.clone())
}

#[derive(Debug, Deserialize)]
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
    let mut session = state.0.lock().expect("career session lock poisoned");
    let response = session.join(request)?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct ReconnectRequest {
    pub reconnect_token: String,
    pub client_versions: VersionSet,
}

async fn reconnect(
    State(state): State<AppState>,
    Json(request): Json<ReconnectRequest>,
) -> Result<Json<JoinResponse>, ApiError> {
    let session = state.0.lock().expect("career session lock poisoned");
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
        let session = state.0.lock().expect("career session lock poisoned");
        session.manager_for_token(&query.reconnect_token)?
    };
    Ok(ws.on_upgrade(move |socket| websocket_loop(socket, state, manager_id)))
}

async fn websocket_loop(mut socket: WebSocket, state: AppState, manager_id: Uuid) {
    let hello = {
        let session = state.0.lock().expect("career session lock poisoned");
        ServerEvent::Hello(HelloBody {
            career_id: session.config.career_id,
            server_versions: session.config.versions.clone(),
            current_revision: session.revision,
        })
    };
    if send_event(&mut socket, hello).await.is_err() {
        return;
    }

    while let Some(Ok(message)) = socket.recv().await {
        let Message::Text(text) = message else {
            continue;
        };
        let events = match serde_json::from_str::<Envelope>(&text) {
            Ok(envelope) => state
                .0
                .lock()
                .expect("career session lock poisoned")
                .apply_command(manager_id, envelope),
            Err(_) => vec![ServerEvent::CommandRejected(CommandRejectedBody {
                command_id: Uuid::nil(),
                current_revision: state.0.lock().expect("career session lock poisoned").revision,
                error: ProtocolError::new(ErrorCode::ProtocolIncompatible),
            })],
        };
        for event in events {
            if send_event(&mut socket, event).await.is_err() {
                return;
            }
        }
    }
}

async fn send_event(socket: &mut WebSocket, event: ServerEvent) -> Result<(), ()> {
    let text = serde_json::to_string(&event).map_err(|_| ())?;
    socket.send(Message::Text(text.into())).await.map_err(|_| ())
}

#[derive(Debug)]
struct CareerSession {
    config: ServerConfig,
    revision: u64,
    slots: HashMap<Uuid, ManagerSlot>,
    claimed_clubs: HashMap<Uuid, Uuid>,
    reconnect_tokens: HashMap<String, Uuid>,
    completed_commands: HashMap<Uuid, Vec<ServerEvent>>,
    ready_managers: HashSet<Uuid>,
}

impl CareerSession {
    fn new(config: ServerConfig) -> Self {
        Self {
            config,
            revision: 0,
            slots: HashMap::new(),
            claimed_clubs: HashMap::new(),
            reconnect_tokens: HashMap::new(),
            completed_commands: HashMap::new(),
            ready_managers: HashSet::new(),
        }
    }

    fn join(&mut self, request: JoinRequest) -> Result<JoinResponse, ApiError> {
        if request.join_secret != self.config.join_secret {
            return Err(ApiError::protocol(ErrorCode::AuthInvalid));
        }
        if !request.client_versions.is_compatible_with(&self.config.versions) {
            return Err(ApiError::protocol(ErrorCode::ProtocolIncompatible));
        }
        if let Some(manager) = self.claimed_clubs.get(&request.club_id) {
            if *manager != request.manager_id {
                return Err(ApiError::protocol(ErrorCode::ClubAlreadyControlled));
            }
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

    fn apply_command(
        &mut self,
        authenticated_manager_id: Uuid,
        envelope: Envelope,
    ) -> Vec<ServerEvent> {
        let command_id = envelope.command_id();
        if let Some(previous) = self.completed_commands.get(&command_id) {
            return previous.clone();
        }
        let event = self.apply_new_command(authenticated_manager_id, envelope);
        self.completed_commands.insert(command_id, event.clone());
        event
    }

    fn apply_new_command(&mut self, manager_id: Uuid, envelope: Envelope) -> Vec<ServerEvent> {
        let command_id = envelope.command_id();
        let reject = |code| {
            ServerEvent::CommandRejected(CommandRejectedBody {
                command_id,
                current_revision: self.revision,
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
                vec![ServerEvent::CommandAck(CommandAckBody {
                    command_id,
                    applied_revision: self.revision,
                }), self.ready_state_event(manager_id)]
            }
            _ => {
                let Some(expected_revision) = envelope.expected_revision else {
                    return vec![reject(ErrorCode::StaleRevision)];
                };
                if expected_revision != self.revision {
                    return vec![reject(ErrorCode::StaleRevision)];
                }
                vec![reject(ErrorCode::MatchCommandNotAllowed)]
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
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

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
}
