use crate::config::Config;
use crate::routes::game::create_router;
use crate::state::state::GameManager;
use axum::{serve};
use http::HeaderValue;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

mod engine;
mod state;
mod handlers;
mod routes;
mod config;
mod utils;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::from_env();

    let cors = if config.allowed_origin == "*" {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_credentials(true)
    } else {
        let allowed_origin =  config.allowed_origin.parse::<HeaderValue>().unwrap();
        CorsLayer::new()
            .allow_origin(allowed_origin)
            .allow_credentials(true)
    };

    let addr: SocketAddr = config
        .server_address
        .parse()
        .expect("Invalid server address format");
    let listener = TcpListener::bind(addr).await.unwrap();
    let game_state = Arc::new(RwLock::new(GameManager::new()));
    let router = create_router(game_state, cors);
    println!("Listening on {}", addr);
    serve(listener, router.into_make_service()).await.unwrap();
    // tokio::net::windows::named_pipe::PipeEnd(&addr).serve(router.into_make_service()).await.unwrap();
}
