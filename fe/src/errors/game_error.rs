use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum GameError {
    CreationFailed(String),
    JoinHttpFailed(String),
    JoinFailed(String),
    SendFailed(String),
}

impl std::error::Error for GameError {}
impl Display for GameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameError::CreationFailed(msg) => write!(f, "Failed to create game: {}", msg),
            GameError::JoinHttpFailed(msg) => write!(f, "Failed to join game: {}", msg),
            GameError::JoinFailed(msg) => write!(f, "Failed to connect to game: {}", msg),
            GameError::SendFailed(msg) => write!(f, "Failed to send payload: {}", msg),
        }
    }
}

