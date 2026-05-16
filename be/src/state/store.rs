use crate::error::AppError;
use crate::state::redis_types::PersistedGameState;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait GameStore: Send + Sync {
    async fn create_game(&self) -> Result<String, AppError>;
    async fn get_game(&self, game_id: &str) -> Result<Option<PersistedGameState>, AppError>;
    async fn save_game(&self, state: &PersistedGameState) -> Result<(), AppError>;
    async fn delete_game(&self, game_id: &str) -> Result<(), AppError>;
    fn game_lock(&self, game_id: &str) -> Arc<tokio::sync::Mutex<()>>;
}
