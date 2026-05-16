use dotenvy::dotenv;
use std::env;

#[derive(Debug)]
pub struct Config {
    pub server_address: String,
    pub allowed_origin: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // Load .env file into environment variables
        dotenv().ok();

        let server_address = env::var("SERVER_ADDRESS").expect("SERVER_ADDRESS is not set");
        let allowed_origin = env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "*".to_string());

        Self { server_address, allowed_origin }
    }
}
