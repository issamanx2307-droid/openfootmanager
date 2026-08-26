use std::net::SocketAddr;

use albion_server::{AppState, ServerConfig, router};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let bind = std::env::var("ALBION_BIND")
        .unwrap_or_else(|_| "127.0.0.1:38421".to_string())
        .parse::<SocketAddr>()
        .expect("ALBION_BIND must be a socket address");
    let join_secret = std::env::var("ALBION_JOIN_SECRET")
        .expect("ALBION_JOIN_SECRET is required for a private server");
    let require_save = matches!(
        std::env::var("ALBION_REQUIRE_SAVE").as_deref(),
        Ok("1" | "true" | "TRUE" | "yes" | "YES")
    );
    let config = match std::env::var("ALBION_SAVE") {
        Ok(path) => ServerConfig::open_save(path, join_secret).expect("open Albion career save"),
        Err(_) if require_save => {
            panic!("ALBION_SAVE is required when ALBION_REQUIRE_SAVE is enabled")
        }
        Err(_) => {
            let career_id = std::env::var("ALBION_CAREER_ID")
                .ok()
                .and_then(|value| Uuid::parse_str(&value).ok())
                .unwrap_or_else(Uuid::new_v4);
            ServerConfig::private_career(career_id, join_secret)
        }
    };
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .expect("bind Albion server");
    eprintln!("Albion server listening on {bind}");
    axum::serve(listener, router(AppState::new(config)))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve Albion server");
    eprintln!("Albion server stopped");
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = terminate.recv() => {},
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
