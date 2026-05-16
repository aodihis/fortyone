use crate::config::Config;
use crate::routes::game::create_router;
use crate::state::state::GameManager;
use axum::Extension;
use http::{HeaderValue, Method};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

mod auth;
mod config;
mod engine;
mod error;
mod handlers;
mod routes;
mod state;
mod utils;

#[tokio::main]
async fn main() {
    // Load .env before tracing so LOG_LEVEL is visible to EnvFilter.
    dotenvy::dotenv().ok();

    // Initialize tracing first — any startup panic will now be logged.
    let filter = EnvFilter::try_from_env("LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = Config::from_env();

    let cors = if config.allowed_origin == "*" {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST])
    } else {
        let origin = config
            .allowed_origin
            .parse::<HeaderValue>()
            .expect("Invalid ALLOWED_ORIGIN value");
        CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([Method::GET, Method::POST])
            .allow_credentials(true)
    };

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(10)
            .finish()
            .expect("Invalid rate limiter config"),
    );

    let addr: SocketAddr = config
        .server_address
        .parse()
        .expect("Invalid SERVER_ADDRESS format");
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    let game_state = Arc::new(RwLock::new(GameManager::new()));
    let jwt_secret = Arc::new(config.jwt_secret.clone());

    let router = create_router(game_state, cors)
        .layer(Extension(jwt_secret))
        .layer(GovernorLayer::new(governor_conf));

    tracing::info!("Listening on {addr}");
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Server failed");
}
