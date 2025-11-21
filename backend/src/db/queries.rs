use crate::models::{User, Game, GamePlayer, GameBoard, GameMove};
use sqlx::{PgPool, Result};
use uuid::Uuid;

// User queries
pub async fn get_user(pool: &PgPool, user_id: i64) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn create_or_update_user(
    pool: &PgPool,
    user_id: i64,
    username: &str,
    avatar_url: Option<&str>,
) -> Result<User> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (user_id, username, avatar_url)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id)
        DO UPDATE SET username = $2, avatar_url = $3, updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(username)
    .bind(avatar_url)
    .fetch_one(pool)
    .await
}

// Game queries
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
