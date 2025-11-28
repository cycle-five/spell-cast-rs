use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub discord: DiscordConfig,
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub game: GameConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub frontend_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub encryption_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameConfig {
    pub dictionary_path: String,
    pub max_players: usize,
    pub default_rounds: u8,
    pub timer_duration: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database = DatabaseConfig {
            url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("DATABASE_MAX_CONNECTIONS must be a number")?,
        };

        let discord = DiscordConfig {
            client_id: env::var("DISCORD_CLIENT_ID")
                .context("DISCORD_CLIENT_ID must be set")?,
            client_secret: env::var("DISCORD_CLIENT_SECRET")
                .context("DISCORD_CLIENT_SECRET must be set")?,
            redirect_uri: env::var("DISCORD_REDIRECT_URI")
                .context("DISCORD_REDIRECT_URI must be set")?,
        };

        let server = ServerConfig {
            host: env::var("HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("PORT must be a number")?,
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        };

        let security = SecurityConfig {
            jwt_secret: env::var("JWT_SECRET")
                .context("JWT_SECRET must be set")?,
            encryption_key: env::var("ENCRYPTION_KEY")
                .context("ENCRYPTION_KEY must be set (32-byte base64 encoded key)")?,
        };

        let game = GameConfig {
            dictionary_path: env::var("DICTIONARY_PATH")
                .unwrap_or_else(|_| "./dictionary.txt".to_string()),
            max_players: env::var("MAX_PLAYERS")
                .unwrap_or_else(|_| "6".to_string())
                .parse()
                .unwrap_or(6),
            default_rounds: env::var("DEFAULT_ROUNDS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            timer_duration: env::var("TIMER_DURATION")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        };

        Ok(Config {
            database,
            discord,
            server,
            security,
            game,
        })
    }

    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
