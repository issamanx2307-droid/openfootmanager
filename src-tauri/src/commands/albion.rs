use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use albion_server::{router, AppState, ServerConfig};
use serde::Serialize;
use tauri::State;

use crate::{AlbionHostState, SaveManagerState};
use ofm_core::state::StateManager;

#[derive(Debug, Serialize)]
pub struct AlbionHostInfo {
    pub server_url: String,
}

/// Starts the private authoritative server for the current saved career.
/// The desktop process owns its lifetime; gameplay traffic itself remains on
/// the server's HTTP/WebSocket interface.
#[tauri::command]
pub async fn start_albion_host(
    state: State<'_, Arc<StateManager>>,
    saves: State<'_, Arc<SaveManagerState>>,
    host: State<'_, Arc<AlbionHostState>>,
    join_secret: String,
) -> Result<AlbionHostInfo, String> {
    if join_secret.trim().is_empty() {
        return Err("be.error.authInvalid".into());
    }
    let game = state.get_game(|game| game.clone()).ok_or("be.error.noActiveGameSession")?;
    let save_id = state.get_save_id().ok_or("be.error.noActiveSaveSession")?;
    let save_path = {
        let mut saves = saves.0.lock().map_err(|_| "be.error.saveManagerUnavailable")?;
        saves.save_game(&game, &save_id)?;
        saves.database_path(&save_id)?
    };
    let config = ServerConfig::open_save(save_path, join_secret)?;
    let listener = tokio::net::TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .await
        .map_err(|_| "be.error.serverStartFailed")?;
    let port = listener.local_addr().map_err(|_| "be.error.serverStartFailed")?.port();
    let mut running = host.0.lock().map_err(|_| "be.error.serverStartFailed")?;
    if let Some(previous) = running.take() {
        previous.abort();
    }
    *running = Some(tokio::spawn(async move {
        let _ = axum::serve(listener, router(AppState::new(config))).await;
    }));
    Ok(AlbionHostInfo { server_url: format!("http://127.0.0.1:{port}") })
}

#[tauri::command]
pub fn stop_albion_host(host: State<'_, Arc<AlbionHostState>>) -> Result<(), String> {
    let mut running = host.0.lock().map_err(|_| "be.error.serverStopFailed")?;
    if let Some(handle) = running.take() {
        handle.abort();
    }
    Ok(())
}
