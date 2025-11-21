use crate::{auth, AppState};
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

// Placeholder test user values for development
// TODO: Remove these once proper Discord OAuth is implemented
const TEST_USER_ID: i64 = 12345;
const TEST_USERNAME: &str = "test_user";

/// Exchange Discord authorization code for access token
pub async fn exchange_code(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeExchangeRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    // TODO: Implement OAuth2 code exchange with Discord
    // For now, create a test token with placeholder user data
    tracing::warn!(
        "OAuth2 code exchange not yet implemented, creating test token for code: {}",
        payload.code
    );

    // Generate a JWT token for testing
    // In production, this should happen after successful Discord OAuth
    let token = auth::generate_token(TEST_USER_ID, TEST_USERNAME, &state.config.security.jwt_secret)
        .map_err(|e| {
            tracing::error!("Failed to generate token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(TokenResponse {
        access_token: token,
    }))
}

/// Get current user info from Discord
pub async fn get_current_user(
    user: auth::AuthenticatedUser,
    State(_state): State<Arc<AppState>>,
) -> Result<Json<UserResponse>, StatusCode> {
    // TODO: Implement user info retrieval from Discord API or database
    tracing::info!(
        "Getting user info for authenticated user: {} ({})",
        user.username,
        user.user_id
    );

    Ok(Json(UserResponse {
        user_id: user.user_id,
        username: user.username,
        avatar_url: None,
    }))
}
