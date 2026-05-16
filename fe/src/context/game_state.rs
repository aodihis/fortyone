use crate::models::players::Player;
use serde::{Deserialize, Serialize};
use yew::Callback;

#[derive(Debug,Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlayerPhase {
    P1,
    P2,
    Waiting,
    GameEnded
}
#[derive(Clone)]
pub enum InGameMovement {
    _Draw,
    _Discard,
    _TakeBin,
    _Close
}

#[derive(Clone)]
pub struct InGameEvent {
    pub _movement: Option<InGameMovement>,
    _from: Option<Player>,
    _to: Option<Player>,
}

#[derive(Clone, PartialEq)]
pub enum GameStatus {
    PreGame,
    InProgress,
    PostGame,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndGameScores {
    pub name: String,
    pub score: i16,
    pub cards: Vec<String>,
}
#[derive(Clone)]
pub struct GameState {
    // pub game_data: Rc<RefCell<GameData>>,
    pub game_status: GameStatus,
    pub game_id: Option<String>,
    pub card_left: u8,
    pub player_index: usize,
    pub player_name: String, // the person name who open the game
    pub current_turn_index: usize, // the turn index
    pub current_turn_phase: PlayerPhase,
    pub players: Vec<Player>,
    pub winner: Option<String>,
    pub _event: Option<InGameEvent>,
    pub counter: usize,
    pub create_game: Callback<String>,
    pub join: Callback<(String, String)>,
    pub disconnect: Callback<()>,
    pub start_game: Callback<()>,
    pub draw: Callback<()>,
    pub take_bin: Callback<()>,
    pub discard: Callback<String>,
    pub close: Callback<String>,
    // pub join: Callback<String>,
    // pub create_game: Callback<()>,
    // pub action: Callback<String>,
}

impl PartialEq for GameState {
    fn eq(&self, other: &Self) -> bool {
        self.counter == other.counter
    }
}

impl GameState {

    pub fn new(create_game: Callback<String>, join: Callback<(String, String)>, disconnect: Callback<()>,
               start_game: Callback<()>, draw: Callback<()>, take_bin: Callback<()>, discard: Callback<String>, close: Callback<String>) -> GameState {
        Self {
            game_status: GameStatus::PreGame,
            game_id: None,
            card_left: 52,
            player_index: 0,
            current_turn_index: 0,
            current_turn_phase: PlayerPhase::P1,
            player_name: "".to_string(),
            players: vec![],
            winner: None,
            _event: None,
            counter: 0,
            create_game,
            join,
            disconnect,
            start_game,
            draw,
            take_bin,
            discard,
            close
        }
    }

    pub fn clear(&mut self) {
        self.game_status = GameStatus::PreGame;
        self.game_id = None;
        self.card_left = 52;
        self.player_index = 0;
        self.current_turn_index = 0;
        self.current_turn_phase = PlayerPhase::P1;
        self.player_name = "".to_string();
        self.players = vec![];
        self.counter = 0;
        self._event = None;
    }
}