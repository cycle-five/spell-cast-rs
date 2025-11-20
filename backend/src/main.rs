mod config;
mod db;
mod dictionary;
mod game;
mod models;
mod routes;
mod utils;
mod websocket;

use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use config::Config;
use dictionary::Dictionary;

/// Application state shared across all handlers
pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub dictionary: Dictionary,
    pub active_games: DashMap<Uuid, GameSession>,
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
    let db = db::create_pool(
        config.database_url(),
        config.database.max_connections,
    )
    .await?;
    tracing::info!("Connected to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db)
        .await?;
    tracing::info!("Database migrations completed");

    // Load dictionary
    let dictionary = match Dictionary::load(&config.game.dictionary_path).await {
        Ok(dict) => {
            tracing::info!("Dictionary loaded successfully");
            dict
        }
        Err(e) => {
            tracing::warn!("Failed to load dictionary: {}. Using empty dictionary for now.", e);
            tracing::warn!("Download a word list to {} for full functionality", config.game.dictionary_path);
            Dictionary::empty()
        }
    };

    // Create application state
    let state = Arc::new(AppState {
        config: config.clone(),
        db,
        dictionary,
        active_games: DashMap::new(),
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // WebSocket endpoint
        .route("/ws", get(websocket::handle_websocket))
        // API routes
        .merge(routes::create_routes(state.clone()))
        // Serve frontend static files
        .nest_service("/", ServeDir::new("../frontend"))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = config.server_addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 Server listening on {}", addr);
    tracing::info!("📝 WebSocket endpoint: ws://{}/ws", addr);
    tracing::info!("🏥 Health check: http://{}/health", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
