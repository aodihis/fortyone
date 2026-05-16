use axum::http::StatusCode;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum GameError {
    #[error("Game not found")]
    GameNotFound,
    #[error("Game already started")]
    GameAlreadyStarted,
    #[error("Not enough players")]
    NotEnoughPlayers,
    #[error("Game is full")]
    GameFull,
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

impl axum::response::IntoResponse for GameError {
    fn into_response(self) -> axum::response::Response {
        let err = match self {
            GameError::GameNotFound => StatusCode::NOT_FOUND,
            GameError::GameAlreadyStarted => StatusCode::BAD_REQUEST,
            GameError::GameFull => StatusCode::BAD_REQUEST,
            GameError::InvalidOperation(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (err, self.to_string()).into_response()
    }
}
