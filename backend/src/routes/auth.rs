use crate::{auth, db, AppState};
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

/// Discord user response from /users/@me endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub avatar: Option<String>,
    pub discriminator: Option<String>,
    pub global_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
}

/// Discord OAuth2 token response
#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    /// Currently unused; not stored because token refresh functionality is not yet implemented.
    /// When implementing token refresh, store this in the database.
    refresh_token: String,
    scope: String,
}

/// Exchange Discord authorization code for access token and create user session
pub async fn exchange_code(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeExchangeRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    tracing::info!("Exchanging authorization code for access token");

    // Step 1: Exchange authorization code for Discord access token
    let discord_token = exchange_code_with_discord(&state, &payload.code)
        .await
        .map_err(|e| {
            tracing::error!("Failed to exchange code with Discord: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    // Step 2: Get user info from Discord API
    let discord_user = get_discord_user_info(&discord_token.access_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Discord user info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Step 3: Parse Discord user ID
    let user_id = discord_user
        .id
        .parse::<i64>()
        .map_err(|e| {
            tracing::error!("Failed to parse Discord user ID: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Step 4: Create or update user in database
    let avatar_url = discord_user.avatar.as_ref().map(|avatar_hash| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            discord_user.id, avatar_hash
        )
    });

    db::queries::create_or_update_user(
        &state.db,
        user_id,
        &discord_user.username,
        avatar_url.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create/update user in database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Successfully authenticated user: {} (ID: {})", discord_user.username, user_id);

    // Step 5: Generate JWT token for our application
    let jwt_token = auth::generate_token(user_id, &discord_user.username, &state.config.security.jwt_secret)
        .map_err(|e| {
            tracing::error!("Failed to generate JWT token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(TokenResponse {
        access_token: jwt_token,
    }))
}

/// Exchange authorization code with Discord OAuth2 API
async fn exchange_code_with_discord(
    state: &AppState,
    code: &str,
) -> anyhow::Result<DiscordTokenResponse> {
    let client = reqwest::Client::new();

    let params = [
        ("client_id", state.config.discord.client_id.as_str()),
        ("client_secret", state.config.discord.client_secret.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", state.config.discord.redirect_uri.as_str()),
    ];

    let response = client
        .post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        tracing::error!("Discord token exchange failed: {} - {}", status, error_text);
        anyhow::bail!("Discord token exchange failed with status {}", status);
    }

    let token_response = response.json::<DiscordTokenResponse>().await?;
    Ok(token_response)
}

/// Get user information from Discord API
async fn get_discord_user_info(access_token: &str) -> anyhow::Result<DiscordUser> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://discord.com/api/users/@me")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        tracing::error!("Discord user info fetch failed: {} - {}", status, error_text);
        anyhow::bail!("Failed to fetch Discord user info with status {}", status);
    }

    let user = response.json::<DiscordUser>().await?;
    Ok(user)
}

/// Get current user info from database
pub async fn get_current_user(
    user: auth::AuthenticatedUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<UserResponse>, StatusCode> {
    tracing::debug!(
        "Getting user info for authenticated user: {} ({})",
        user.username,
        user.user_id
    );

    // Fetch user from database
    let db_user = db::queries::get_user(&state.db, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("User not found in database: {}", user.user_id);
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(UserResponse {
        user_id: db_user.user_id,
        username: db_user.username,
        avatar_url: db_user.avatar_url,
    }))
}
