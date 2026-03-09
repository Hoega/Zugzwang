use std::env;

pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("ZUGZWANG_DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .expect("ZUGZWANG_DATABASE_URL (or DATABASE_URL) must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
        }
    }
}
