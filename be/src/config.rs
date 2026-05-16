use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_address: String,
    pub allowed_origin: String,
    pub jwt_secret: String,
    pub redis_url: String,
    pub redis_key_prefix: String,
    pub reconnect_timeout_secs: u64,
}

impl Config {
    /// Reads config from environment. Call `dotenvy::dotenv().ok()` before this.
    pub fn from_env() -> Self {
        let server_address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| {
            tracing::warn!("SERVER_ADDRESS not set, defaulting to 127.0.0.1:3000");
            "127.0.0.1:3000".to_string()
        });
        let allowed_origin = env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "*".to_string());
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::error!("JWT_SECRET is not set — server will not issue valid tokens");
            String::new()
        });
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| {
            tracing::warn!("REDIS_URL not set, defaulting to redis://127.0.0.1/");
            "redis://127.0.0.1/".to_string()
        });
        let redis_key_prefix =
            env::var("REDIS_KEY_PREFIX").unwrap_or_else(|_| "fortyone".to_string());
        let reconnect_timeout_secs = env::var("RECONNECT_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120u64);

        Self {
            server_address,
            allowed_origin,
            jwt_secret,
            redis_url,
            redis_key_prefix,
            reconnect_timeout_secs,
        }
    }
}
