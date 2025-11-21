use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum GameMode {
    Multiplayer,
    #[serde(rename = "2v2")]
    TwoVTwo,
    Adventure,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum GameState {
    Waiting,
    Active,
    Finished,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Game {
    pub game_id: Uuid,
    pub guild_id: Option<i64>,
    pub channel_id: i64,
    pub game_mode: GameMode,
    pub state: GameState,
    pub current_round: i32,
    pub max_rounds: i32,
    pub current_turn_player: Option<i64>,
    pub timer_enabled: bool,
    pub timer_duration: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GamePlayer {
    pub id: i32,
    pub game_id: Uuid,
    pub user_id: i64,
    pub team: Option<i32>,
    pub score: i32,
    pub is_bot: bool,
    pub bot_difficulty: Option<String>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameBoard {
    pub game_id: Uuid,
    pub grid: serde_json::Value,
    pub used_words: serde_json::Value,
    pub round_number: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameMove {
    pub id: i32,
    pub game_id: Uuid,
    pub user_id: i64,
    pub round_number: i32,
    pub word: String,
    pub score: i32,
    pub positions: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Multiplier {
    #[serde(rename = "DL")]
    DoubleLetter,
    #[serde(rename = "TL")]
    TripleLetter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub letter: char,
    pub value: u8,
    pub multiplier: Option<Multiplier>,
}

pub type Grid = Vec<Vec<GridCell>>;
