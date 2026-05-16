use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum GameError {
    CreationFailed(String),
    SendFailed(String),
    JoinFailed(String),
    // Add other error variants as needed
}

impl std::error::Error for GameError {}
impl Display for GameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameError::CreationFailed(msg) => write!(f, "Failed to create game: {}", msg),
            GameError::JoinFailed(msg) => write!(f, "Failed to join game: {}", msg),
            GameError::SendFailed(msg) => write!(f, "Failed to send payload: {}", msg)
        }
    }
}

