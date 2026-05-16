use crate::error::AppError;
use crate::state::redis_types::PersistedGameState;
use crate::state::store::GameStore;
use axum::extract::ws::Message;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GameStateStatus {
    Lobby,
    InProgress,
    Finished,
}

type SenderMap = Arc<DashMap<String, DashMap<Uuid, (String, UnboundedSender<Message>)>>>;
type ReconnectTimerMap = Arc<DashMap<String, DashMap<Uuid, AbortHandle>>>;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn GameStore>,
    pub senders: SenderMap,
    pub reconnect_timers: ReconnectTimerMap,
    pub reconnect_timeout_secs: u64,
}

impl AppState {
    pub fn new(store: Arc<dyn GameStore>, reconnect_timeout_secs: u64) -> Self {
        Self {
            store,
            senders: Arc::new(DashMap::new()),
            reconnect_timers: Arc::new(DashMap::new()),
            reconnect_timeout_secs,
        }
    }

    pub async fn create_game(&self) -> Result<String, AppError> {
        self.store.create_game().await
    }

    pub async fn get_game(&self, game_id: &str) -> Result<Option<PersistedGameState>, AppError> {
        self.store.get_game(game_id).await
    }

    pub async fn save_game(&self, state: &PersistedGameState) -> Result<(), AppError> {
        self.store.save_game(state).await
    }

    pub async fn delete_game(&self, game_id: &str) -> Result<(), AppError> {
        self.store.delete_game(game_id).await
    }

    pub fn game_lock(&self, game_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.store.game_lock(game_id)
    }
}
