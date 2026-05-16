use crate::engine::game::Game;
use crate::state::state::GameStateStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistedGameState {
    pub id: String,
    pub status: GameStateStatus,
    pub game: Option<Game>,
    /// (player_id, player_name) — senders are not serializable and live in AppState.senders
    pub players: Vec<(Uuid, String)>,
}
