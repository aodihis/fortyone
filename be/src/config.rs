use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_address: String,
    pub allowed_origin: String,
    pub jwt_secret: String,
}

impl Config {
    /// Reads config from environment. Call `dotenvy::dotenv().ok()` before this.
    pub fn from_env() -> Self {
        let server_address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| {
            tracing::warn!("SERVER_ADDRESS not set, defaulting to 127.0.0.1:3000");
            "127.0.0.1:3000".to_string()
        });
        let allowed_origin =
            env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "*".to_string());
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::error!("JWT_SECRET is not set — server will not issue valid tokens");
            String::new()
        });

        Self {
            server_address,
            allowed_origin,
            jwt_secret,
        }
    }
}
