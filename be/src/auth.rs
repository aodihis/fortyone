use crate::error::AppError;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TOKEN_EXPIRY_SECS: u64 = 86_400; // 24 hours

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub game_id: String,
    pub player_id: String,
    pub player_name: String,
    pub exp: u64,
}

pub fn create_token(
    game_id: &str,
    player_id: Uuid,
    player_name: &str,
    secret: &str,
) -> Result<String, AppError> {
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + TOKEN_EXPIRY_SECS;

    let claims = Claims {
        game_id: game_id.to_owned(),
        player_id: player_id.to_string(),
        player_name: player_name.to_owned(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!("JWT encode error: {e}");
        AppError::Internal
    })
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| {
        tracing::warn!("JWT validation failed: {e}");
        AppError::Unauthorized
    })
}
