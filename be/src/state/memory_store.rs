use crate::error::AppError;
use crate::state::redis_types::PersistedGameState;
use crate::state::state::GameStateStatus;
use crate::state::store::GameStore;
use crate::utils::generate_short_uuid;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(dead_code)]
pub struct MemoryGameStore {
    games: Arc<RwLock<std::collections::HashMap<String, PersistedGameState>>>,
    game_locks: Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl MemoryGameStore {
    pub fn new() -> Self {
        Self {
            games: Arc::new(RwLock::new(std::collections::HashMap::new())),
            game_locks: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait]
impl GameStore for MemoryGameStore {
    async fn create_game(&self) -> Result<String, AppError> {
        let game_id = generate_short_uuid();
        let initial = PersistedGameState {
            id: game_id.clone(),
            status: GameStateStatus::Lobby,
            game: None,
            players: vec![],
        };
        self.games.write().await.insert(game_id.clone(), initial);
        Ok(game_id)
    }

    async fn get_game(&self, game_id: &str) -> Result<Option<PersistedGameState>, AppError> {
        Ok(self.games.read().await.get(game_id).cloned())
    }

    async fn save_game(&self, state: &PersistedGameState) -> Result<(), AppError> {
        self.games
            .write()
            .await
            .insert(state.id.clone(), state.clone());
        Ok(())
    }

    async fn delete_game(&self, game_id: &str) -> Result<(), AppError> {
        self.games.write().await.remove(game_id);
        self.game_locks.remove(game_id);
        Ok(())
    }

    fn game_lock(&self, game_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.game_locks
            .entry(game_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}
