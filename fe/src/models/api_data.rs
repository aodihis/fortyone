use crate::context::game_state::PlayerPhase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    PlayerJoin,
    PlayerLeft,
    GameEvent,
    EndGame,
    Reply
}


#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ConnectResponse {
    pub game_id: String,
    pub num_of_players: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameResponse {
    pub message_type: MessageType,
    pub status: String,
    pub data: Option<GameData>,
    pub message: Option<String>
}



#[derive(Debug, Serialize, Deserialize)]
pub struct GameData {
    pub player_id: Option<String>,
    pub player_pos: Option<u8>,
    pub num_of_players: Option<u8>,
    pub card_left: Option<u8>,
    pub current_turn: Option<u8>,
    pub current_phase: Option<PlayerPhase>,
    pub event: Option<GameEvent>,
    pub players: Vec<PlayerData>,
    pub winner_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PlayerData {
    pub name: String,
    #[serde(default)]
    pub bin: Vec<String>,
    pub hand: Vec<String>,
    #[serde(default)]
    pub score: i16,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum GameEventType {
    GameStart,
    Draw,
    TakeBin,
    Discard,
    Close,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameEvent {
    event_type: GameEventType,
    from: Option<u8>,
    to: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GameRequestAction {
    StartGame,
    Draw,
    TakeBin,
    Discard,
    Close,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestPayload {
    pub action: GameRequestAction,
    pub card: Option<String>,
}


