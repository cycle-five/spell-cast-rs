use crate::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

/// Validate Discord authorization code
fn validate_auth_code(code: &str) -> Result<(), &'static str> {
    // Check if code is empty
    if code.is_empty() {
        return Err("Authorization code cannot be empty");
    }

    // Check reasonable length (Discord codes are typically 30-40 characters)
    // Allow some flexibility but prevent extremely long inputs
    if code.len() < 10 || code.len() > 100 {
        return Err("Authorization code has invalid length");
    }

    // Check that code contains only valid characters
    // Discord codes are alphanumeric with possible hyphens and underscores
    if !code.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Authorization code contains invalid characters");
    }

    Ok(())
}

/// Exchange Discord authorization code for access token
pub async fn exchange_code(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<CodeExchangeRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    // Validate the authorization code
    if let Err(_) = validate_auth_code(&payload.code) {
        tracing::warn!("Invalid authorization code received");
        return Err(StatusCode::BAD_REQUEST);
    }

    // TODO: Implement OAuth2 code exchange with Discord
    // For now, return a placeholder
    tracing::warn!("OAuth2 code exchange not yet implemented");

    Ok(Json(TokenResponse {
        access_token: "placeholder_token".to_string(),
    }))
}

/// Get current user info from Discord
pub async fn get_current_user(
    State(_state): State<Arc<AppState>>,
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
