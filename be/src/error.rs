use axum::http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Game not found")]
    GameNotFound,
    #[error("Game already started")]
    GameAlreadyStarted,
    #[allow(dead_code)]
    #[error("Not enough players")]
    NotEnoughPlayers,
    #[error("Game is full")]
    GameFull,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal error")]
    Internal,
    #[error("Storage error")]
    Redis(#[from] deadpool_redis::PoolError),
    #[error("Storage error")]
    RedisCmd(#[from] redis::RedisError),
    #[error("Serialization error")]
    Json(#[from] serde_json::Error),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::GameNotFound => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::GameAlreadyStarted | AppError::GameFull | AppError::InvalidInput(_) => {
                StatusCode::BAD_REQUEST
            }
            AppError::NotEnoughPlayers => StatusCode::BAD_REQUEST,
            AppError::Internal | AppError::Redis(_) | AppError::RedisCmd(_) | AppError::Json(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        tracing::warn!("Request error: {self}");
        (status, self.to_string()).into_response()
    }
}
