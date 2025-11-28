mod auth;
mod config;
mod db;
mod dictionary;
mod encryption;
mod game;
mod models;
mod routes;
mod utils;
mod websocket;

use anyhow::Result;
use axum::{routing::get, Router};
use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use config::Config;
use dictionary::Dictionary;
use std::time::{Duration, Instant};
use websocket::messages::{LobbyType, ServerMessage};

/// Grace period before removing disconnected players (seconds)
pub const PLAYER_DISCONNECT_GRACE_PERIOD: Duration = Duration::from_secs(60);
/// Grace period before removing empty lobbies (seconds)
pub const LOBBY_EMPTY_GRACE_PERIOD: Duration = Duration::from_secs(120);
/// Allowed characters for lobby codes - excludes I, O, 0, 1 for readability
pub const LOBBY_CODE_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// Length of generated lobby codes
pub const LOBBY_CODE_LENGTH: usize = 6;

/// Connection state for a lobby player
#[derive(Debug, Clone)]
pub enum PlayerConnectionState {
    /// Player is actively connected with an open WebSocket
    Connected,
    /// Player's WebSocket dropped, waiting for reconnection within grace period
    /// Player is still visible in the lobby during this state
    AwaitingReconnect { since: Instant },
}

/// Information about a connected lobby player
#[derive(Debug, Clone)]
pub struct LobbyPlayer {
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub tx: mpsc::Sender<ServerMessage>,
    pub connection_state: PlayerConnectionState,
}

impl LobbyPlayer {
    /// Returns true if the player has an active WebSocket connection
    pub fn is_connected(&self) -> bool {
        matches!(self.connection_state, PlayerConnectionState::Connected)
    }

    /// Returns true if the player should be visible in the lobby
    /// (both connected and awaiting reconnect players are visible)
    pub fn is_visible(&self) -> bool {
        // Players are visible in both Connected and AwaitingReconnect states
        // They only become invisible when removed by the background cleanup task
        true
    }
}

/// A game lobby that players can join
#[derive(Debug)]
pub struct Lobby {
    pub lobby_id: String,
    pub lobby_type: LobbyType,
    /// For custom lobbies, a short shareable code (e.g., "ABC123")
    pub lobby_code: Option<String>,
    /// For channel lobbies, the Discord channel ID
    pub channel_id: Option<String>,
    /// For channel lobbies, the Discord guild ID
    pub guild_id: Option<String>,
    /// Players in the lobby, keyed by user_id
    pub players: DashMap<i64, LobbyPlayer>,
    /// When the lobby was created
    pub created_at: Instant,
    /// When the lobby became empty (for cleanup grace period)
    pub empty_since: Option<Instant>,
}

impl Lobby {
    /// Create a new channel-based lobby
    pub fn new_channel(channel_id: String, guild_id: Option<String>) -> Self {
        Self {
            lobby_id: format!("channel:{}", channel_id),
            lobby_type: LobbyType::Channel,
            lobby_code: None,
            channel_id: Some(channel_id),
            guild_id,
            players: DashMap::new(),
            created_at: Instant::now(),
            empty_since: None,
        }
    }

    /// Create a new custom lobby with a generated code
    pub fn new_custom() -> Self {
        let lobby_code = generate_lobby_code();
        Self {
            lobby_id: format!("custom:{}", lobby_code),
            lobby_type: LobbyType::Custom,
            lobby_code: Some(lobby_code),
            channel_id: None,
            guild_id: None,
            players: DashMap::new(),
            created_at: Instant::now(),
            empty_since: None,
        }
    }

    /// Count of actively connected players (excludes disconnected ones in grace period)
    pub fn connected_player_count(&self) -> usize {
        self.players.iter().filter(|p| p.is_connected()).count()
    }

    /// Check if lobby has any players (connected or disconnected in grace period)
    pub fn has_any_players(&self) -> bool {
        !self.players.is_empty()
    }
}

/// Generate a short, readable lobby code (6 alphanumeric characters)
fn generate_lobby_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..LOBBY_CODE_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..LOBBY_CODE_CHARSET.len());
            LOBBY_CODE_CHARSET[idx] as char
        })
        .collect()
}

/// Application state shared across all handlers
pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub dictionary: Dictionary,
    pub active_games: DashMap<Uuid, GameSession>,
    /// All lobbies keyed by lobby_id (e.g., "channel:123" or "custom:ABC123")
    pub lobbies: DashMap<String, Lobby>,
    /// Index from lobby_code to lobby_id for quick custom lobby lookup
    pub lobby_code_index: DashMap<String, String>,
    pub http_client: reqwest::Client,
}

/// In-memory game session data
pub struct GameSession {
    pub game_id: Uuid,
    pub players: Vec<i64>,
    // TODO: Add more game session data
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spell_cast_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Spell Cast backend server...");

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Connect to database
    let db = db::create_pool(config.database_url(), config.database.max_connections).await?;
    tracing::info!("Connected to database");

    // Run migrations
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("Database migrations completed");

    // Load dictionary
    let dictionary = match Dictionary::load(&config.game.dictionary_path).await {
        Ok(dict) => {
            tracing::info!("Dictionary loaded successfully");
            dict
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load dictionary: {}. Using empty dictionary for now.",
                e
            );
            tracing::warn!(
                "Download a word list to {} for full functionality",
                config.game.dictionary_path
            );
            Dictionary::empty()
        }
    };

    // Create shared HTTP client for reusing connections
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    tracing::info!("HTTP client initialized");

    // Create application state
    let state = Arc::new(AppState {
        config: config.clone(),
        db,
        dictionary,
        active_games: DashMap::new(),
        lobbies: DashMap::new(),
        lobby_code_index: DashMap::new(),
        http_client,
    });

    // Spawn background task to clean up stale players and empty lobbies
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        lobby_cleanup_task(cleanup_state).await;
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Serve frontend static files
    let frontend_service = ServeDir::new("../frontend");

    // Build router
    let app = Router::new()
        // WebSocket endpoint
        .route("/ws", get(websocket::handle_websocket))
        // API routes
        .merge(routes::create_routes())
        // Serve frontend at /play and static assets at root
        //.nest_service("/play", frontend_service.clone())
        .fallback_service(frontend_service)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = config.server_addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);
    tracing::info!("WebSocket endpoint: ws://{}/ws", addr);
    tracing::info!("Health check: http://{}/health", addr);
    tracing::info!("Game frontend: http://{}/", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Background task that periodically cleans up stale disconnected players and empty lobbies
async fn lobby_cleanup_task(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(15));

    loop {
        interval.tick().await;

        let now = Instant::now();
        let mut lobbies_to_remove = Vec::new();
        let mut players_to_remove: Vec<(String, i64)> = Vec::new();

        // Scan all lobbies
        for lobby_ref in state.lobbies.iter() {
            let lobby_id = lobby_ref.key().clone();
            let lobby = lobby_ref.value();

            // Find players that have exceeded the grace period
            for player_ref in lobby.players.iter() {
                if let PlayerConnectionState::AwaitingReconnect { since } =
                    &player_ref.connection_state
                {
                    if now.duration_since(*since) > PLAYER_DISCONNECT_GRACE_PERIOD {
                        players_to_remove.push((lobby_id.clone(), player_ref.user_id));
                    }
                }
            }

            // Check if lobby should be removed (empty beyond grace period)
            if let Some(empty_since) = lobby.empty_since {
                if now.duration_since(empty_since) > LOBBY_EMPTY_GRACE_PERIOD {
                    lobbies_to_remove.push(lobby_id.clone());
                }
            }
        }

        // Remove stale players
        for (lobby_id, user_id) in players_to_remove {
            if let Some(lobby) = state.lobbies.get(&lobby_id) {
                lobby.players.remove(&user_id);
                tracing::info!(
                    "Removed stale disconnected player {} from lobby {} (grace period expired)",
                    user_id,
                    lobby_id
                );
            }
        }

        // Remove stale lobbies
        for lobby_id in lobbies_to_remove {
            if let Some((_, lobby)) = state.lobbies.remove(&lobby_id) {
                // Remove from code index if custom lobby
                if let Some(code) = lobby.lobby_code {
                    state.lobby_code_index.remove(&code);
                }
                tracing::info!(
                    "Removed empty lobby {} (grace period expired)",
                    lobby_id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lobby_code_length() {
        // Generate multiple codes and verify they are always 6 characters
        for _ in 0..100 {
            let code = generate_lobby_code();
            assert_eq!(
                code.len(),
                LOBBY_CODE_LENGTH,
                "Generated lobby code '{}' should be exactly {} characters",
                code,
                LOBBY_CODE_LENGTH
            );
        }
    }

    #[test]
    fn test_generate_lobby_code_charset() {
        // Generate multiple codes and verify all characters are from allowed charset
        for _ in 0..100 {
            let code = generate_lobby_code();
            for c in code.chars() {
                assert!(
                    LOBBY_CODE_CHARSET.contains(&(c as u8)),
                    "Character '{}' in code '{}' is not in allowed charset",
                    c,
                    code
                );
            }
        }
    }

    #[test]
    fn test_generate_lobby_code_uppercase() {
        // Generate multiple codes and verify all alphabetic characters are uppercase
        for _ in 0..100 {
            let code = generate_lobby_code();
            for c in code.chars() {
                if c.is_alphabetic() {
                    assert!(
                        c.is_uppercase(),
                        "Character '{}' in code '{}' should be uppercase",
                        c,
                        code
                    );
                }
            }
        }
    }
}
