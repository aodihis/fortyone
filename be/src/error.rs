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

/// Returns (HTTP status, user-facing message) for a Redis command error.
pub fn classify_redis_error(e: &redis::RedisError) -> (StatusCode, &'static str) {
    match e.kind() {
        redis::ErrorKind::IoError => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Cannot reach the storage backend — please try again shortly",
        ),
        redis::ErrorKind::ResponseError => {
            let detail = e.to_string();
            if detail.contains("OOM") || detail.contains("maxmemory") || e.code() == Some("OOM") {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Server storage is at capacity — please try again later",
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage error — please try again",
                )
            }
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Storage error — please try again",
        ),
    }
}

/// Returns (HTTP status, user-facing message) for a pool error.
pub fn classify_pool_error(e: &deadpool_redis::PoolError) -> (StatusCode, &'static str) {
    // PoolError::Backend(redis::RedisError) — delegate to redis classifier
    // PoolError::Timeout — pool exhausted or Redis slow to respond
    // PoolError::Closed — server shutting down
    // Others — generic unavailable
    let s = e.to_string().to_lowercase();
    if s.contains("timeout") {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Storage connection pool timed out — please try again shortly",
        )
    } else if s.contains("oom") || s.contains("maxmemory") {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Server storage is at capacity — please try again later",
        )
    } else if s.contains("io") || s.contains("connection") || s.contains("refused") {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Cannot reach the storage backend — please try again shortly",
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable — please try again",
        )
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match &self {
            AppError::GameNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::GameAlreadyStarted
            | AppError::GameFull
            | AppError::InvalidInput(_)
            | AppError::NotEnoughPlayers => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Internal => {
                tracing::error!("Internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::Redis(e) => {
                let (status, msg) = classify_pool_error(e);
                tracing::error!("Redis pool error: {e}");
                (status, msg.to_string())
            }
            AppError::RedisCmd(e) => {
                let (status, msg) = classify_redis_error(e);
                tracing::error!("Redis command error [{:?}]: {e}", e.kind());
                (status, msg.to_string())
            }
            AppError::Json(e) => {
                tracing::error!("Serialization error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };
        (status, body).into_response()
    }
}
