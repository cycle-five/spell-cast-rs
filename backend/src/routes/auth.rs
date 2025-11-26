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
    /// Unique username (e.g., "username" or "username#0")
    pub username: String,
    pub avatar: Option<String>,
    /// Display name shown in Discord UI (preferred for display)
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
    #[allow(dead_code)]
    token_type: String,
    expires_in: i64,
    /// Stored in database for token refresh functionality
    refresh_token: String,
    #[allow(dead_code)]
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
    let discord_user = get_discord_user_info(&discord_token.access_token, &state.http_client)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Discord user info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Step 3: Parse Discord user ID
    // Discord IDs are u64 snowflakes, but we store as i64 in the database
    // Parse as u64 first to handle all valid Discord IDs, then cast to i64
    // Note: Very large IDs (> i64::MAX) will wrap to negative, but remain unique
    let user_id = discord_user
        .id
        .parse::<u64>()
        .map_err(|e| {
            tracing::error!("Failed to parse Discord user ID: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })? as i64;

    // Step 4: Create or update user in database
    let avatar_url = discord_user.avatar.as_ref().map(|avatar_hash| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            discord_user.id, avatar_hash
        )
    });

    // Calculate token expiration time (Discord tokens expire in expires_in seconds)
    let token_expires_at = chrono::Utc::now() + chrono::Duration::seconds(discord_token.expires_in);

    db::queries::create_or_update_user(
        &state.db,
        user_id,
        &discord_user.username,
        discord_user.global_name.as_deref(),
        avatar_url.as_deref(),
        Some(&discord_token.refresh_token),
        Some(token_expires_at),
        &state.config.security.encryption_key,
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
    let params = [
        ("client_id", state.config.discord.client_id.as_str()),
        ("client_secret", state.config.discord.client_secret.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", state.config.discord.redirect_uri.as_str()),
    ];

    let response = state.http_client
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
async fn get_discord_user_info(
    access_token: &str,
    http_client: &reqwest::Client,
) -> anyhow::Result<DiscordUser> {
    let response = http_client
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

/// Refresh Discord OAuth2 token using a refresh token
async fn refresh_discord_token(
    state: &AppState,
    refresh_token: &str,
) -> anyhow::Result<DiscordTokenResponse> {
    let params = [
        ("client_id", state.config.discord.client_id.as_str()),
        ("client_secret", state.config.discord.client_secret.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let response = state.http_client
        .post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        tracing::error!("Discord token refresh failed: {} - {}", status, error_text);
        anyhow::bail!("Discord token refresh failed with status {}", status);
    }

    let token_response = response.json::<DiscordTokenResponse>().await?;
    Ok(token_response)
}

/// Revoke a Discord OAuth2 token
async fn revoke_discord_token(
    state: &AppState,
    token: &str,
) -> anyhow::Result<()> {
    let params = [
        ("client_id", state.config.discord.client_id.as_str()),
        ("client_secret", state.config.discord.client_secret.as_str()),
        ("token", token),
    ];

    let response = state.http_client
        .post("https://discord.com/api/oauth2/token/revoke")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        tracing::warn!("Discord token revocation returned non-success: {} - {}", status, error_text);
        // Don't fail on revocation errors - token may already be invalid
    }

    Ok(())
}

/// Refresh the user's OAuth2 tokens and return a new JWT
///
/// This endpoint:
/// 1. Retrieves the stored encrypted refresh token from the database
/// 2. Uses it to get new access/refresh tokens from Discord
/// 3. Stores the new refresh token (token rotation)
/// 4. Returns a fresh JWT for the application
pub async fn refresh_token(
    user: auth::AuthenticatedUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<TokenResponse>, StatusCode> {
    tracing::info!("Refreshing token for user: {} ({})", user.username, user.user_id);

    // Step 1: Get user with their encrypted refresh token from database
    let db_user = db::queries::get_user(&state.db, user.user_id, &state.config.security.encryption_key)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching user for refresh: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("User not found for token refresh: {}", user.user_id);
            StatusCode::NOT_FOUND
        })?;

    // Step 2: Ensure we have a refresh token
    let current_refresh_token = db_user.refresh_token.ok_or_else(|| {
        tracing::warn!("No refresh token stored for user: {}", user.user_id);
        StatusCode::UNAUTHORIZED
    })?;

    // Step 3: Refresh with Discord
    let discord_token = refresh_discord_token(&state, &current_refresh_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to refresh Discord token: {}", e);
            // If refresh fails, user needs to re-authenticate
            StatusCode::UNAUTHORIZED
        })?;

    // Step 4: Calculate new expiration time
    let token_expires_at = chrono::Utc::now() + chrono::Duration::seconds(discord_token.expires_in);

    // Step 5: Store the new refresh token (token rotation)
    db::queries::update_user_refresh_token(
        &state.db,
        user.user_id,
        &discord_token.refresh_token,
        token_expires_at,
        &state.config.security.encryption_key,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to update refresh token in database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Successfully refreshed token for user: {} ({})", user.username, user.user_id);

    // Step 6: Generate new JWT
    let jwt_token = auth::generate_token(user.user_id, &user.username, &state.config.security.jwt_secret)
        .map_err(|e| {
            tracing::error!("Failed to generate JWT token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(TokenResponse {
        access_token: jwt_token,
    }))
}

/// Revoke the user's Discord OAuth2 tokens and clear from database
///
/// This endpoint:
/// 1. Retrieves the stored refresh token
/// 2. Revokes it with Discord's API
/// 3. Clears all tokens from the database
pub async fn revoke_token(
    user: auth::AuthenticatedUser,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Revoking tokens for user: {} ({})", user.username, user.user_id);

    // Get user with their refresh token
    let db_user = db::queries::get_user(&state.db, user.user_id, &state.config.security.encryption_key)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching user for revoke: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // If we have a refresh token, revoke it with Discord
    if let Some(user) = db_user {
        if let Some(refresh_token) = user.refresh_token {
            if let Err(e) = revoke_discord_token(&state, &refresh_token).await {
                tracing::warn!("Failed to revoke token with Discord (continuing anyway): {}", e);
            }
        }
    }

    // Clear tokens from database regardless of Discord API result
    db::queries::clear_user_tokens(&state.db, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clear tokens from database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Successfully revoked tokens for user: {} ({})", user.username, user.user_id);
    Ok(StatusCode::NO_CONTENT)
}

/// Log out the user by clearing their stored tokens (without Discord revocation)
///
/// Use this for a simple logout that doesn't require contacting Discord.
/// For a full logout that also revokes the token with Discord, use /revoke.
pub async fn logout(
    user: auth::AuthenticatedUser,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Logging out user: {} ({})", user.username, user.user_id);

    db::queries::clear_user_tokens(&state.db, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clear tokens for logout: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Successfully logged out user: {} ({})", user.username, user.user_id);
    Ok(StatusCode::NO_CONTENT)
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
    let db_user = db::queries::get_user(&state.db, user.user_id, &state.config.security.encryption_key)
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
