use crate::handlers::game::{create_game, join_game, ws_handler};
use crate::state::state::GameManager;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

pub fn create_router(state: Arc<RwLock<GameManager>>, cors_layer: CorsLayer) -> Router {
    Router::new()
        .route("/create", get(create_game))
        .route("/{game_id}/join", post(join_game))
        .route("/{game_id}/ws", get(ws_handler))
        .with_state(state)
        .layer(cors_layer)
}
