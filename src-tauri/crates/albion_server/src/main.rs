use std::net::SocketAddr;

use albion_server::{router, AppState, ServerConfig};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let bind = std::env::var("ALBION_BIND")
        .unwrap_or_else(|_| "127.0.0.1:38421".to_string())
        .parse::<SocketAddr>()
        .expect("ALBION_BIND must be a socket address");
    let join_secret = std::env::var("ALBION_JOIN_SECRET")
        .expect("ALBION_JOIN_SECRET is required for a private server");
    let career_id = std::env::var("ALBION_CAREER_ID")
        .ok()
        .and_then(|value| Uuid::parse_str(&value).ok())
        .unwrap_or_else(Uuid::new_v4);
    let listener = tokio::net::TcpListener::bind(bind).await.expect("bind Albion server");
    axum::serve(listener, router(AppState::new(ServerConfig::private_career(career_id, join_secret))))
        .await
        .expect("serve Albion server");
}
