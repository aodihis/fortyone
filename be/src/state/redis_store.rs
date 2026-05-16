use crate::error::AppError;
use crate::state::redis_types::PersistedGameState;
use crate::state::state::GameStateStatus;
use crate::state::store::GameStore;
use crate::utils::generate_short_uuid;
use async_trait::async_trait;
use dashmap::DashMap;
use deadpool_redis::Pool as RedisPool;
use std::sync::Arc;

const TTL_SECS: u64 = 3600;

pub struct RedisGameStore {
    pool: RedisPool,
    key_prefix: String,
    game_locks: Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl RedisGameStore {
    pub fn new(pool: RedisPool, key_prefix: String) -> Self {
        Self {
            pool,
            key_prefix,
            game_locks: Arc::new(DashMap::new()),
        }
    }

    fn game_key(&self, game_id: &str) -> String {
        format!("{}:game:{}", self.key_prefix, game_id)
    }
}

#[async_trait]
impl GameStore for RedisGameStore {
    async fn create_game(&self) -> Result<String, AppError> {
        let game_id = generate_short_uuid();
        let initial = PersistedGameState {
            id: game_id.clone(),
            status: GameStateStatus::Lobby,
            game: None,
            players: vec![],
        };
        self.save_game(&initial).await?;
        Ok(game_id)
    }

    async fn get_game(&self, game_id: &str) -> Result<Option<PersistedGameState>, AppError> {
        let mut conn = self.pool.get().await?;
        let key = self.game_key(game_id);
        let raw: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut *conn)
            .await?;
        match raw {
            None => Ok(None),
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        }
    }

    async fn save_game(&self, state: &PersistedGameState) -> Result<(), AppError> {
        let mut conn = self.pool.get().await?;
        let key = self.game_key(&state.id);
        let json = serde_json::to_string(state)?;
        redis::cmd("SET")
            .arg(&key)
            .arg(json)
            .arg("EX")
            .arg(TTL_SECS)
            .query_async::<()>(&mut *conn)
            .await?;
        tracing::debug!(key = %key, "Game saved to Redis");
        Ok(())
    }

    async fn delete_game(&self, game_id: &str) -> Result<(), AppError> {
        let mut conn = self.pool.get().await?;
        let key = self.game_key(game_id);
        redis::cmd("DEL")
            .arg(&key)
            .query_async::<()>(&mut *conn)
            .await?;
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
