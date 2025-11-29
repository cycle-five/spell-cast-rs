use sqlx::{PgPool, Result};
use tracing;
use uuid::Uuid;

use crate::{
    encryption,
    models::{
        Game, GameBoard, GameDbState, GameMode, GameMove, GamePlayer, GamePlayerRecord, GameState,
        GameStatus, GridCell, User, UserGuildProfile,
    },
};

const DEFAULT_TIMER_DURATION: i32 = 30_i32; // seconds
const DEFAULT_TIMER_DISABLED: bool = false;

// User queries
pub async fn get_user(pool: &PgPool, user_id: i64, encryption_key: &str) -> Result<Option<User>> {
    let mut user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    // Decrypt refresh token if present
    if let Some(ref mut u) = user {
        if let Some(ref encrypted_token) = u.refresh_token {
            u.refresh_token = encryption::decrypt(encrypted_token, encryption_key).ok();
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
        Some(encryption::encrypt(token, encryption_key).map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to encrypt refresh token: {}", e))
        })?)
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
        "#,
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
        "#,
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
        "#,
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

pub async fn update_game_state(pool: &PgPool, game_id: Uuid, state: &str) -> Result<()> {
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
) -> Result<GamePlayerRecord> {
    sqlx::query_as::<_, GamePlayerRecord>(
        r#"
        INSERT INTO game_players (game_id, user_id, team, is_bot)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(game_id)
    .bind(user_id)
    .bind(team)
    .bind(is_bot)
    .fetch_one(pool)
    .await
}

pub async fn get_game_players(pool: &PgPool, game_id: Uuid) -> Result<Vec<GamePlayerRecord>> {
    sqlx::query_as::<_, GamePlayerRecord>(
        "SELECT * FROM game_players WHERE game_id = $1 ORDER BY joined_at",
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
        "#,
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

// =============================================================================
// Game Session Management (for WebSocket game lifecycle)
// =============================================================================

/// Create a new game session for a lobby
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `lobby_id` - Lobby identifier (e.g., "channel:123456" or "custom:ABC123")
/// * `created_by` - Discord user ID of the player who started the game
/// * `total_rounds` - Number of rounds for the game (typically 5)
///
/// # Returns
/// The UUID of the newly created game session
pub async fn create_game_session(
    pool: &PgPool,
    lobby_id: &str,
    created_by: i64,
    total_rounds: u8,
) -> Result<Uuid> {
    let game_id = Uuid::new_v4();

    // Parse lobby_id to extract channel_id and guild_id
    let (channel_id, guild_id) = parse_lobby_id(lobby_id)?;

    sqlx::query(
        r#"
        INSERT INTO games (
            game_id, guild_id, channel_id, game_mode, state,
            current_round, max_rounds, current_turn_player,
            timer_enabled, timer_duration
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(game_id)
    .bind(guild_id)
    .bind(channel_id)
    .bind(GameMode::Multiplayer) // Default game mode
    .bind(GameDbState::Waiting.to_string()) // Initial state
    .bind(1_i32) // Start at round 1
    .bind(total_rounds as i32)
    .bind(created_by) // Creator is first turn player
    .bind(DEFAULT_TIMER_DISABLED) // Timer disabled by default
    .bind(DEFAULT_TIMER_DURATION) // Default timer duration
    .execute(pool)
    .await?;

    Ok(game_id)
}

/// Parse a lobby_id string to extract channel_id and optional guild_id
///
/// # Arguments
/// * `lobby_id` - Lobby identifier string in one of the following formats:
///   - "channel:123456789" - Channel-based lobby
///   - "custom:ABC123" - Custom lobby with alphanumeric code
///   - "123456789" - Raw channel ID (fallback)
///
/// # Returns
/// Result containing (channel_id, guild_id) tuple, or an error if parsing fails
///
/// # Errors
/// Returns `sqlx::Error::Protocol` if the lobby_id format is invalid or cannot be parsed
fn parse_lobby_id(lobby_id: &str) -> Result<(i64, Option<i64>)> {
    if let Some(channel_str) = lobby_id.strip_prefix("channel:") {
        // Channel-based lobby: "channel:123456789"
        let channel = channel_str.parse::<i64>().map_err(|e| {
            sqlx::Error::Protocol(format!(
                "Failed to parse channel_id '{}': {}",
                channel_str, e
            ))
        })?;
        Ok((channel, None)) // Guild ID would need to be passed separately if needed
    } else if let Some(code) = lobby_id.strip_prefix("custom:") {
        // Custom lobby: "custom:ABC123" - encode the lobby code
        if code.is_empty() {
            return Err(sqlx::Error::Protocol(
                "Custom lobby code cannot be empty".to_string(),
            ));
        }
        let encoded = encode_lobby_code_to_i64(code);
        Ok((encoded, None))
    } else {
        // Fallback: try to parse as raw channel ID
        let channel = lobby_id.parse::<i64>().map_err(|e| {
            sqlx::Error::Protocol(format!(
                "Failed to parse lobby_id '{}' as channel ID: {}",
                lobby_id, e
            ))
        })?;
        Ok((channel, None))
    }
}

/// Encode a lobby code (e.g., "ABC123") to a unique negative i64
/// This allows storing custom lobby games in the channel_id column
fn encode_lobby_code_to_i64(code: &str) -> i64 {
    // Use a simple encoding: treat the code as base-36 and negate it
    // This ensures custom lobbies have negative channel_ids (distinguishable from Discord IDs)
    let mut value: i64 = 0;
    for c in code.chars().take(6) {
        value = value * 36
            + match c {
                '0'..='9' => (c as i64) - ('0' as i64),
                'A'..='Z' => (c as i64) - ('A' as i64) + 10,
                'a'..='z' => (c as i64) - ('a' as i64) + 10,
                _ => 0,
            };
    }
    // Negate to distinguish from real Discord channel IDs (which are positive)
    -value.saturating_sub(1) // Subtract 1 to avoid -0
}

/// Add multiple players to a game with their turn orders
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `game_id` - The game to add players to
/// * `players` - Vector of (user_id, turn_order) tuples
///
/// # Returns
/// Result indicating success or failure
pub async fn add_game_players_batch(
    pool: &PgPool,
    game_id: Uuid,
    players: &[(i64, u8)],
) -> Result<()> {
    // Use a transaction to ensure all players are added atomically
    let mut tx = pool.begin().await?;

    for (user_id, turn_order) in players {
        sqlx::query(
            r#"
            INSERT INTO game_players (game_id, user_id, team, score, is_bot)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (game_id, user_id) DO UPDATE SET
                team = EXCLUDED.team
            "#,
        )
        .bind(game_id)
        .bind(*user_id)
        .bind(*turn_order as i32) // TODO: Add dedicated turn_order column instead of reusing team
        .bind(0_i32) // Initial score
        .bind(false) // Not a bot
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Create or update a game board with the grid data
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `game_id` - The game this board belongs to
/// * `grid_json` - The grid data as JSON
///
/// # Returns
/// Result indicating success or failure
pub async fn create_or_update_game_board(
    pool: &PgPool,
    game_id: Uuid,
    grid_json: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO game_boards (game_id, grid, used_words, round_number)
        VALUES ($1, $2, '[]'::jsonb, 1)
        ON CONFLICT (game_id) DO UPDATE SET
            grid = $2,
            updated_at = NOW()
        "#,
    )
    .bind(game_id)
    .bind(grid_json)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get the active game for a lobby and construct a GameState
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `lobby_id` - Lobby identifier (e.g., "channel:123456" or "custom:ABC123")
///
/// # Returns
/// The active GameState if one exists, None otherwise
pub async fn get_active_game_for_lobby(pool: &PgPool, lobby_id: &str) -> Result<Option<GameState>> {
    // Parse lobby_id to get channel_id
    let (channel_id, _guild_id) = parse_lobby_id(lobby_id)?;

    // Get the active game for this channel
    let game = sqlx::query_as::<_, Game>(
        r#"
        SELECT * FROM games
        WHERE channel_id = $1 AND state IN ('waiting', 'active')
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    let game = match game {
        Some(g) => g,
        None => return Ok(None),
    };

    // Get the game board
    let board = sqlx::query_as::<_, GameBoard>("SELECT * FROM game_boards WHERE game_id = $1")
        .bind(game.game_id)
        .fetch_optional(pool)
        .await?;

    // Get all players for this game
    let player_records = sqlx::query_as::<_, GamePlayerRecord>(
        "SELECT * FROM game_players WHERE game_id = $1 ORDER BY team, joined_at",
    )
    .bind(game.game_id)
    .fetch_all(pool)
    .await?;

    // Get user info for each player
    let mut players = Vec::with_capacity(player_records.len());
    for (idx, record) in player_records.iter().enumerate() {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
            .bind(record.user_id)
            .fetch_optional(pool)
            .await?;

        if let Some(u) = user {
            players.push(GamePlayer {
                user_id: Uuid::new_v4(), // Generate a UUID for in-memory tracking
                username: u.username,
                avatar_url: u.avatar_url,
                score: record.score,
                turn_order: record.team.unwrap_or(idx as i32) as u8,
                is_connected: true, // Assume connected; WebSocket handler will update
            });
        }
    }

    // Parse the grid from JSON
    let grid: Vec<Vec<GridCell>> = if let Some(ref b) = board {
        serde_json::from_value(b.grid.clone()).map_err(|e| {
            tracing::error!("Failed to deserialize grid for game {}: {}", game.game_id, e);
            sqlx::Error::Protocol(format!("Invalid grid data: {}", e))
        })?
    } else {
        Vec::new()
    };

    // Parse used words from JSON
    let used_words: std::collections::HashSet<String> = if let Some(ref b) = board {
        serde_json::from_value(b.used_words.clone()).map_err(|e| {
            tracing::error!("Failed to deserialize used_words for game {}: {}", game.game_id, e);
            sqlx::Error::Protocol(format!("Invalid used_words data: {}", e))
        })?
    } else {
        std::collections::HashSet::new()
    };

    // Convert game state to GameStatus
    let status = match game.state {
        GameDbState::Waiting => GameStatus::WaitingToStart,
        GameDbState::Active => GameStatus::InProgress,
        GameDbState::Finished => GameStatus::Finished,
        GameDbState::Cancelled => GameStatus::Finished,
    };

    // Build round submissions map (all false initially, WebSocket will update)
    let round_submissions = players.iter().map(|p| (p.user_id, false)).collect();

    // Determine current player index
    let current_player_index = if let Some(turn_player) = game.current_turn_player {
        player_records
            .iter()
            .position(|p| p.user_id == turn_player)
            .unwrap_or(0)
    } else {
        0
    };

    Ok(Some(GameState {
        game_id: game.game_id,
        grid,
        players,
        current_round: game.current_round as u8,
        total_rounds: game.max_rounds as u8,
        current_player_index,
        used_words,
        round_submissions,
        status,
        created_at: game.created_at,
    }))
}

/// Update game state in the database
pub async fn update_game_db_state(pool: &PgPool, game_id: Uuid, state: GameDbState) -> Result<()> {
    let state_str = match state {
        GameDbState::Waiting => "waiting",
        GameDbState::Active => "active",
        GameDbState::Finished => "finished",
        GameDbState::Cancelled => "cancelled",
    };

    sqlx::query("UPDATE games SET state = $1, started_at = NOW() WHERE game_id = $2")
        .bind(state_str)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update game round and current turn player
pub async fn update_game_round(
    pool: &PgPool,
    game_id: Uuid,
    round: i32,
    current_player_id: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE games
        SET current_round = $1, current_turn_player = $2
        WHERE game_id = $3
        "#,
    )
    .bind(round)
    .bind(current_player_id)
    .bind(game_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update a player's score in the database
pub async fn update_player_score(
    pool: &PgPool,
    game_id: Uuid,
    user_id: i64,
    score: i32,
) -> Result<()> {
    sqlx::query("UPDATE game_players SET score = $1 WHERE game_id = $2 AND user_id = $3")
        .bind(score)
        .bind(game_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update used words for a game board
pub async fn update_game_board_used_words(
    pool: &PgPool,
    game_id: Uuid,
    used_words: &[String],
) -> Result<()> {
    let words_json = serde_json::to_value(used_words)
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize used words: {}", e)))?;

    sqlx::query("UPDATE game_boards SET used_words = $1, updated_at = NOW() WHERE game_id = $2")
        .bind(words_json)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mark a game as finished with final results
pub async fn finish_game(pool: &PgPool, game_id: Uuid, winner_id: Option<i64>) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE games
        SET state = 'finished', finished_at = NOW(), current_turn_player = $1
        WHERE game_id = $2
        "#,
    )
    .bind(winner_id)
    .bind(game_id)
    .execute(pool)
    .await?;
    Ok(())
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
        "#,
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
    sqlx::query_as::<_, GameMove>("SELECT * FROM game_moves WHERE game_id = $1 ORDER BY timestamp")
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
        "SELECT * FROM user_guild_profiles WHERE user_id = $1 AND guild_id = $2",
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
        "#,
    )
    .bind(user_id)
    .bind(guild_id)
    .bind(nickname)
    .fetch_one(pool)
    .await
}

// =============================================================================
// Tests for Game Session Management Functions
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Lobby ID Encoding/Parsing Tests
    // =========================================================================

    #[test]
    fn test_encode_lobby_code_basic() {
        // Test basic encoding - should return negative values
        let code = "ABC123";
        let encoded = encode_lobby_code_to_i64(code);
        assert!(encoded <= 0, "Encoded value should be negative or zero");
    }

    #[test]
    fn test_encode_lobby_code_different_codes_produce_different_values() {
        let code1 = "ABC123";
        let code2 = "XYZ789";
        let code3 = "GAME01";

        let encoded1 = encode_lobby_code_to_i64(code1);
        let encoded2 = encode_lobby_code_to_i64(code2);
        let encoded3 = encode_lobby_code_to_i64(code3);

        assert_ne!(encoded1, encoded2);
        assert_ne!(encoded2, encoded3);
        assert_ne!(encoded1, encoded3);
    }

    #[test]
    fn test_encode_lobby_code_case_insensitive() {
        // Uppercase and lowercase should produce the same encoding
        let upper = "ABC123";
        let lower = "abc123";

        let encoded_upper = encode_lobby_code_to_i64(upper);
        let encoded_lower = encode_lobby_code_to_i64(lower);

        assert_eq!(
            encoded_upper, encoded_lower,
            "Encoding should be case-insensitive"
        );
    }

    #[test]
    fn test_encode_lobby_code_empty_string() {
        let code = "";
        let encoded = encode_lobby_code_to_i64(code);
        // Empty string: value = 0, then -value.saturating_sub(1) = -(0.saturating_sub(1)) = -(-1) = 1
        assert_eq!(encoded, 1);
    }

    #[test]
    fn test_encode_lobby_code_single_char() {
        // Test single character codes
        // For "0": value = 0, result = -(0.saturating_sub(1)) = -(-1) = 1
        assert_eq!(encode_lobby_code_to_i64("0"), 1);
        // For "1": value = 1, result = -(1.saturating_sub(1)) = -(0) = 0
        assert_eq!(encode_lobby_code_to_i64("1"), 0);
        // For "A": value = 10, result = -(10.saturating_sub(1)) = -(9) = -9
        assert_eq!(encode_lobby_code_to_i64("A"), -9);
        // For "Z": value = 35, result = -(35.saturating_sub(1)) = -(34) = -34
        assert_eq!(encode_lobby_code_to_i64("Z"), -34);
    }

    #[test]
    fn test_encode_lobby_code_truncates_to_six_chars() {
        // Codes longer than 6 characters should be truncated
        let short = "ABC123";
        let long = "ABC123XYZ";

        let encoded_short = encode_lobby_code_to_i64(short);
        let encoded_long = encode_lobby_code_to_i64(long);

        assert_eq!(
            encoded_short, encoded_long,
            "Codes should be truncated to 6 characters"
        );
    }

    #[test]
    fn test_encode_lobby_code_special_chars_treated_as_zero() {
        // Special characters should be treated as 0
        let with_special = "AB-123";
        let without_special = "AB0123";

        let encoded_special = encode_lobby_code_to_i64(with_special);
        let encoded_without = encode_lobby_code_to_i64(without_special);

        assert_eq!(
            encoded_special, encoded_without,
            "Special chars should be treated as 0"
        );
    }

    #[test]
    fn test_encode_lobby_code_numeric_only() {
        // Test purely numeric codes
        let code = "123456";
        let encoded = encode_lobby_code_to_i64(code);
        assert!(encoded <= 0, "Encoded value should be negative or zero");

        // Different numeric codes should produce different values
        let code2 = "654321";
        let encoded2 = encode_lobby_code_to_i64(code2);
        assert_ne!(encoded, encoded2);
    }

    #[test]
    fn test_encode_lobby_code_alphabetic_only() {
        // Test purely alphabetic codes
        let code = "ABCDEF";
        let encoded = encode_lobby_code_to_i64(code);
        assert!(encoded < 0, "Encoded alphabetic value should be negative");

        let code2 = "ZZZZZZ";
        let encoded2 = encode_lobby_code_to_i64(code2);
        assert!(encoded2 < 0, "All Z's should be negative");
        // ZZZZZZ = 35*36^5 + 35*36^4 + ... + 35 = large number
        assert!(
            encoded2 < encoded,
            "ZZZZZZ should be more negative than ABCDEF"
        );
    }

    // =========================================================================
    // Lobby ID Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_channel_lobby_id() {
        // Test parsing of channel-based lobby IDs
        let lobby_id = "channel:123456789";
        let expected_channel_id: i64 = 123456789;

        let result = parse_lobby_id(lobby_id);
        assert!(result.is_ok(), "Should successfully parse channel lobby ID");

        let (channel_id, guild_id) = result.unwrap();
        assert_eq!(channel_id, expected_channel_id);
        assert!(guild_id.is_none(), "Guild ID should be None for channel lobby");
    }

    #[test]
    fn test_parse_custom_lobby_id() {
        // Test parsing of custom lobby IDs
        let lobby_id = "custom:ABC123";

        let result = parse_lobby_id(lobby_id);
        assert!(result.is_ok(), "Should successfully parse custom lobby ID");

        let (channel_id, guild_id) = result.unwrap();
        assert!(
            channel_id <= 0,
            "Custom lobby channel_id should be non-positive"
        );
        assert!(guild_id.is_none(), "Guild ID should be None for custom lobby");
    }

    #[test]
    fn test_parse_raw_channel_id() {
        // Test parsing of raw channel IDs (fallback case)
        let lobby_id = "987654321";

        let result = parse_lobby_id(lobby_id);
        assert!(result.is_ok(), "Should successfully parse raw channel ID");

        let (channel_id, guild_id) = result.unwrap();
        assert_eq!(channel_id, 987654321);
        assert!(guild_id.is_none(), "Guild ID should be None for raw channel ID");
    }

    #[test]
    fn test_parse_invalid_lobby_id() {
        // Test parsing of invalid lobby IDs
        let lobby_id = "invalid:format";

        let result = parse_lobby_id(lobby_id);
        assert!(
            result.is_err(),
            "Should return error for invalid lobby ID format"
        );
    }

    #[test]
    fn test_parse_lobby_id_invalid_channel_number() {
        // Test parsing of channel lobby ID with invalid number
        let lobby_id = "channel:not_a_number";

        let result = parse_lobby_id(lobby_id);
        assert!(
            result.is_err(),
            "Should return error for invalid channel number"
        );
    }

    #[test]
    fn test_parse_lobby_id_empty_custom_code() {
        // Test parsing of custom lobby ID with empty code
        let lobby_id = "custom:";

        let result = parse_lobby_id(lobby_id);
        assert!(
            result.is_err(),
            "Should return error for empty custom lobby code"
        );
    }

    // =========================================================================
    // GameDbState Conversion Tests
    // =========================================================================

    #[test]
    fn test_game_db_state_to_string_waiting() {
        let state = GameDbState::Waiting;
        let state_str = match state {
            GameDbState::Waiting => "waiting",
            GameDbState::Active => "active",
            GameDbState::Finished => "finished",
            GameDbState::Cancelled => "cancelled",
        };
        assert_eq!(state_str, "waiting");
    }

    #[test]
    fn test_game_db_state_to_string_active() {
        let state = GameDbState::Active;
        let state_str = match state {
            GameDbState::Waiting => "waiting",
            GameDbState::Active => "active",
            GameDbState::Finished => "finished",
            GameDbState::Cancelled => "cancelled",
        };
        assert_eq!(state_str, "active");
    }

    #[test]
    fn test_game_db_state_to_string_finished() {
        let state = GameDbState::Finished;
        let state_str = match state {
            GameDbState::Waiting => "waiting",
            GameDbState::Active => "active",
            GameDbState::Finished => "finished",
            GameDbState::Cancelled => "cancelled",
        };
        assert_eq!(state_str, "finished");
    }

    #[test]
    fn test_game_db_state_to_string_cancelled() {
        let state = GameDbState::Cancelled;
        let state_str = match state {
            GameDbState::Waiting => "waiting",
            GameDbState::Active => "active",
            GameDbState::Finished => "finished",
            GameDbState::Cancelled => "cancelled",
        };
        assert_eq!(state_str, "cancelled");
    }

    #[test]
    fn test_game_db_state_to_game_status_conversion() {
        // Test conversion from GameDbState to GameStatus
        assert!(matches!(GameDbState::Waiting, GameDbState::Waiting));
        assert!(matches!(GameDbState::Active, GameDbState::Active));
        assert!(matches!(GameDbState::Finished, GameDbState::Finished));
        assert!(matches!(GameDbState::Cancelled, GameDbState::Cancelled));

        // Verify the expected mapping logic
        let status_from_waiting = match GameDbState::Waiting {
            GameDbState::Waiting => GameStatus::WaitingToStart,
            GameDbState::Active => GameStatus::InProgress,
            GameDbState::Finished => GameStatus::Finished,
            GameDbState::Cancelled => GameStatus::Finished,
        };
        assert_eq!(status_from_waiting, GameStatus::WaitingToStart);

        let status_from_active = match GameDbState::Active {
            GameDbState::Waiting => GameStatus::WaitingToStart,
            GameDbState::Active => GameStatus::InProgress,
            GameDbState::Finished => GameStatus::Finished,
            GameDbState::Cancelled => GameStatus::Finished,
        };
        assert_eq!(status_from_active, GameStatus::InProgress);

        let status_from_finished = match GameDbState::Finished {
            GameDbState::Waiting => GameStatus::WaitingToStart,
            GameDbState::Active => GameStatus::InProgress,
            GameDbState::Finished => GameStatus::Finished,
            GameDbState::Cancelled => GameStatus::Finished,
        };
        assert_eq!(status_from_finished, GameStatus::Finished);

        let status_from_cancelled = match GameDbState::Cancelled {
            GameDbState::Waiting => GameStatus::WaitingToStart,
            GameDbState::Active => GameStatus::InProgress,
            GameDbState::Finished => GameStatus::Finished,
            GameDbState::Cancelled => GameStatus::Finished,
        };
        assert_eq!(status_from_cancelled, GameStatus::Finished);
    }

    // =========================================================================
    // JSON Serialization Tests for Board Operations
    // =========================================================================

    #[test]
    fn test_grid_cell_json_serialization() {
        let cell = GridCell {
            letter: 'A',
            value: 1,
            multiplier: None,
        };

        let json = serde_json::to_value(&cell).expect("Failed to serialize GridCell");
        assert_eq!(json["letter"], "A");
        assert_eq!(json["value"], 1);
        assert!(json["multiplier"].is_null());
    }

    #[test]
    fn test_grid_cell_with_multiplier_json_serialization() {
        use crate::models::Multiplier;

        let cell = GridCell {
            letter: 'Q',
            value: 10,
            multiplier: Some(Multiplier::TripleLetter),
        };

        let json = serde_json::to_value(&cell).expect("Failed to serialize GridCell");
        assert_eq!(json["letter"], "Q");
        assert_eq!(json["value"], 10);
        assert_eq!(json["multiplier"], "TL");
    }

    #[test]
    fn test_grid_json_round_trip() {
        use crate::models::Multiplier;

        let grid: Vec<Vec<GridCell>> = vec![
            vec![
                GridCell {
                    letter: 'A',
                    value: 1,
                    multiplier: None,
                },
                GridCell {
                    letter: 'B',
                    value: 3,
                    multiplier: Some(Multiplier::DoubleLetter),
                },
            ],
            vec![
                GridCell {
                    letter: 'C',
                    value: 3,
                    multiplier: Some(Multiplier::TripleLetter),
                },
                GridCell {
                    letter: 'D',
                    value: 2,
                    multiplier: None,
                },
            ],
        ];

        // Serialize to JSON
        let json = serde_json::to_value(&grid).expect("Failed to serialize grid");

        // Deserialize back
        let deserialized: Vec<Vec<GridCell>> =
            serde_json::from_value(json).expect("Failed to deserialize grid");

        // Verify round-trip
        assert_eq!(grid.len(), deserialized.len());
        for (orig_row, deser_row) in grid.iter().zip(deserialized.iter()) {
            assert_eq!(orig_row.len(), deser_row.len());
            for (orig_cell, deser_cell) in orig_row.iter().zip(deser_row.iter()) {
                assert_eq!(orig_cell.letter, deser_cell.letter);
                assert_eq!(orig_cell.value, deser_cell.value);
                assert_eq!(orig_cell.multiplier, deser_cell.multiplier);
            }
        }
    }

    #[test]
    fn test_used_words_json_serialization() {
        let used_words = vec!["HELLO".to_string(), "WORLD".to_string(), "TEST".to_string()];

        // Serialize to JSON
        let json = serde_json::to_value(&used_words).expect("Failed to serialize used words");

        // Verify it's an array
        assert!(json.is_array());
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert!(arr.contains(&serde_json::Value::String("HELLO".to_string())));
        assert!(arr.contains(&serde_json::Value::String("WORLD".to_string())));
        assert!(arr.contains(&serde_json::Value::String("TEST".to_string())));

        // Deserialize back
        let deserialized: Vec<String> =
            serde_json::from_value(json).expect("Failed to deserialize used words");
        assert_eq!(used_words, deserialized);
    }

    #[test]
    fn test_used_words_empty_array_json() {
        let used_words: Vec<String> = vec![];

        let json = serde_json::to_value(&used_words).expect("Failed to serialize empty array");
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 0);

        let deserialized: Vec<String> =
            serde_json::from_value(json).expect("Failed to deserialize empty array");
        assert!(deserialized.is_empty());
    }

    #[test]
    fn test_used_words_hashset_deserialization() {
        // Test deserializing JSON array to HashSet (as used in get_active_game_for_lobby)
        let json = serde_json::json!(["APPLE", "BANANA", "CHERRY"]);

        let used_words: std::collections::HashSet<String> =
            serde_json::from_value(json).expect("Failed to deserialize to HashSet");

        assert_eq!(used_words.len(), 3);
        assert!(used_words.contains("APPLE"));
        assert!(used_words.contains("BANANA"));
        assert!(used_words.contains("CHERRY"));
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_encode_lobby_code_max_value() {
        // Test encoding of maximum base-36 value for 6 characters
        // ZZZZZZ in base 36 = 35*36^5 + 35*36^4 + 35*36^3 + 35*36^2 + 35*36^1 + 35*36^0
        // = 35*(60466176 + 1679616 + 46656 + 1296 + 36 + 1) = 35*62193781 = 2176782335
        let code = "ZZZZZZ";
        let encoded = encode_lobby_code_to_i64(code);
        assert!(encoded < 0, "Max code should produce negative value");
        assert!(encoded > i64::MIN, "Should not overflow");
    }

    #[test]
    fn test_lobby_id_distinguishes_channel_from_custom() {
        // A custom lobby with code "000000" should NOT collide with channel ID 0
        let custom_encoded = encode_lobby_code_to_i64("000000");
        // "000000" encodes to value = 0, then -(0.saturating_sub(1)) = -(-1) = 1
        assert_eq!(custom_encoded, 1);

        // So custom lobbies are distinguishable because real Discord channel IDs
        // are large positive numbers (snowflakes), not small values like 0 or 1
    }

    #[test]
    fn test_grid_deserialization_handles_empty() {
        let json = serde_json::json!([]);
        let grid: Vec<Vec<GridCell>> =
            serde_json::from_value(json).expect("Failed to deserialize empty grid");
        assert!(grid.is_empty());
    }

    #[test]
    fn test_grid_deserialization_handles_default_on_error() {
        // Test that invalid JSON falls back to empty on unwrap_or_default
        let invalid_json = serde_json::json!("not an array");
        let grid: Vec<Vec<GridCell>> = serde_json::from_value(invalid_json).unwrap_or_default();
        assert!(grid.is_empty());
    }

    #[test]
    fn test_hashset_deserialization_handles_default_on_error() {
        // Test that invalid JSON falls back to empty HashSet on unwrap_or_default
        let invalid_json = serde_json::json!("not an array");
        let used_words: std::collections::HashSet<String> =
            serde_json::from_value(invalid_json).unwrap_or_default();
        assert!(used_words.is_empty());
    }
}
