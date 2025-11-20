use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CodeExchangeRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
}

/// Exchange Discord authorization code for access token
pub async fn exchange_code(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeExchangeRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    // TODO: Implement OAuth2 code exchange with Discord
    // For now, return a placeholder
    tracing::warn!("OAuth2 code exchange not yet implemented");

    Ok(Json(TokenResponse {
        access_token: "placeholder_token".to_string(),
    }))
}

/// Get current user info from Discord
pub async fn get_current_user(
    State(state): State<Arc<AppState>>,
    // TODO: Extract bearer token from headers
) -> Result<Json<UserResponse>, StatusCode> {
    // TODO: Implement user info retrieval from Discord API
    tracing::warn!("User info retrieval not yet implemented");

    Ok(Json(UserResponse {
        user_id: 0,
        username: "placeholder".to_string(),
        avatar_url: None,
    }))
}
