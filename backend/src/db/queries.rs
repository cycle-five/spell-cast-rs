use crate::{encryption, models::{User, Game, GamePlayer, GameBoard, GameMove, UserGuildProfile}};
use sqlx::{PgPool, Result};
use uuid::Uuid;

// User queries
pub async fn get_user(pool: &PgPool, user_id: i64, encryption_key: &str) -> Result<Option<User>> {
    let mut user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    // Decrypt refresh token if present
    if let Some(ref mut u) = user {
        if let Some(ref encrypted_token) = u.refresh_token {
            u.refresh_token = encryption::decrypt(encrypted_token, encryption_key)
                .ok();
        }
    }

    Ok(user)
}

pub async fn create_or_update_user(
    pool: &PgPool,
    user_id: i64,
    username: &str,
    global_name: Option<&str>,
    avatar_url: Option<&str>,
    refresh_token: Option<&str>,
    token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    encryption_key: &str,
) -> Result<User> {
    // Encrypt refresh token if present
    let encrypted_token = if let Some(token) = refresh_token {
        Some(encryption::encrypt(token, encryption_key)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to encrypt refresh token: {}", e)))?)
    } else {
        None
    };

    let mut user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (user_id, username, global_name, avatar_url, refresh_token, token_expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (user_id)
        DO UPDATE SET
            username = $2,
            global_name = $3,
            avatar_url = $4,
            refresh_token = $5,
            token_expires_at = $6,
            updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(username)
    .bind(global_name)
    .bind(avatar_url)
    .bind(encrypted_token.as_deref())
    .bind(token_expires_at)
    .fetch_one(pool)
    .await?;

    // Decrypt refresh token for the returned user
    if let Some(ref encrypted) = user.refresh_token {
        user.refresh_token = encryption::decrypt(encrypted, encryption_key).ok();
    }

    Ok(user)
}

/// Update a user's refresh token and expiration time
///
/// This is used for token rotation - when we refresh with Discord,
/// we get a new refresh token that should replace the old one.
pub async fn update_user_refresh_token(
    pool: &PgPool,
    user_id: i64,
    refresh_token: &str,
    token_expires_at: chrono::DateTime<chrono::Utc>,
    encryption_key: &str,
) -> Result<()> {
    // Encrypt the new refresh token
    let encrypted_token = encryption::encrypt(refresh_token, encryption_key)
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to encrypt refresh token: {}", e)))?;

    sqlx::query(
        r#"
        UPDATE users
        SET refresh_token = $1,
            token_expires_at = $2,
            updated_at = NOW()
        WHERE user_id = $3
        "#
    )
    .bind(&encrypted_token)
    .bind(token_expires_at)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Clear a user's OAuth tokens from the database
///
/// Used for logout and token revocation operations.
pub async fn clear_user_tokens(pool: &PgPool, user_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE users
        SET refresh_token = NULL,
            token_expires_at = NULL,
            updated_at = NOW()
        WHERE user_id = $1
        "#
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

// Game queries
// TODO: Game logic not yet fully implemented - these will be used when game state management is added
#[allow(dead_code)]
pub async fn create_game(pool: &PgPool, game: &Game) -> Result<Game> {
    sqlx::query_as::<_, Game>(
        r#"
        INSERT INTO games (
            game_id, guild_id, channel_id, game_mode, state,
            current_round, max_rounds, timer_enabled, timer_duration
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(game.game_id)
    .bind(game.guild_id)
    .bind(game.channel_id)
    .bind(&game.game_mode)
    .bind(&game.state)
    .bind(game.current_round)
    .bind(game.max_rounds)
    .bind(game.timer_enabled)
    .bind(game.timer_duration)
    .fetch_one(pool)
    .await
}

pub async fn get_game(pool: &PgPool, game_id: Uuid) -> Result<Option<Game>> {
    sqlx::query_as::<_, Game>("SELECT * FROM games WHERE game_id = $1")
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_game_state(
    pool: &PgPool,
    game_id: Uuid,
    state: &str,
) -> Result<()> {
    sqlx::query("UPDATE games SET state = $1 WHERE game_id = $2")
        .bind(state)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

// Game player queries
pub async fn add_player_to_game(
    pool: &PgPool,
    game_id: Uuid,
    user_id: i64,
    team: Option<i32>,
    is_bot: bool,
) -> Result<GamePlayer> {
    sqlx::query_as::<_, GamePlayer>(
        r#"
        INSERT INTO game_players (game_id, user_id, team, is_bot)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#
    )
    .bind(game_id)
    .bind(user_id)
    .bind(team)
    .bind(is_bot)
    .fetch_one(pool)
    .await
}

pub async fn get_game_players(pool: &PgPool, game_id: Uuid) -> Result<Vec<GamePlayer>> {
    sqlx::query_as::<_, GamePlayer>(
        "SELECT * FROM game_players WHERE game_id = $1 ORDER BY joined_at"
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

// Game board queries
pub async fn create_game_board(
    pool: &PgPool,
    game_id: Uuid,
    grid: serde_json::Value,
) -> Result<GameBoard> {
    sqlx::query_as::<_, GameBoard>(
        r#"
        INSERT INTO game_boards (game_id, grid)
        VALUES ($1, $2)
        RETURNING *
        "#
    )
    .bind(game_id)
    .bind(grid)
    .fetch_one(pool)
    .await
}

pub async fn get_game_board(pool: &PgPool, game_id: Uuid) -> Result<Option<GameBoard>> {
    sqlx::query_as::<_, GameBoard>("SELECT * FROM game_boards WHERE game_id = $1")
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

// Game move queries
pub async fn create_game_move(
    pool: &PgPool,
    game_id: Uuid,
    user_id: i64,
    round_number: i32,
    word: &str,
    score: i32,
    positions: serde_json::Value,
) -> Result<GameMove> {
    sqlx::query_as::<_, GameMove>(
        r#"
        INSERT INTO game_moves (game_id, user_id, round_number, word, score, positions)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(game_id)
    .bind(user_id)
    .bind(round_number)
    .bind(word)
    .bind(score)
    .bind(positions)
    .fetch_one(pool)
    .await
}

pub async fn get_game_moves(pool: &PgPool, game_id: Uuid) -> Result<Vec<GameMove>> {
    sqlx::query_as::<_, GameMove>(
        "SELECT * FROM game_moves WHERE game_id = $1 ORDER BY timestamp"
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

// User guild profile queries
#[allow(dead_code)]
pub async fn get_user_guild_profile(
    pool: &PgPool,
    user_id: i64,
    guild_id: i64,
) -> Result<Option<UserGuildProfile>> {
    sqlx::query_as::<_, UserGuildProfile>(
        "SELECT * FROM user_guild_profiles WHERE user_id = $1 AND guild_id = $2"
    )
    .bind(user_id)
    .bind(guild_id)
    .fetch_optional(pool)
    .await
}

#[allow(dead_code)]
pub async fn create_or_update_guild_profile(
    pool: &PgPool,
    user_id: i64,
    guild_id: i64,
    nickname: Option<&str>,
) -> Result<UserGuildProfile> {
    sqlx::query_as::<_, UserGuildProfile>(
        r#"
        INSERT INTO user_guild_profiles (user_id, guild_id, nickname)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, guild_id)
        DO UPDATE SET
            nickname = $3,
            updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(guild_id)
    .bind(nickname)
    .fetch_one(pool)
    .await
}
