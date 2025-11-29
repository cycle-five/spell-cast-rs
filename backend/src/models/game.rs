use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// =============================================================================
// Database Models (for SQLx persistence)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum GameMode {
    Multiplayer,
    #[serde(rename = "2v2")]
    TwoVTwo,
    Adventure,
}

/// Database persistence state for games
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum GameDbState {
    Waiting,
    Active,
    Finished,
    Cancelled,
}

impl std::fmt::Display for GameDbState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Match database schema values: 'waiting', 'active', 'finished', 'cancelled'
        match self {
            GameDbState::Waiting => write!(f, "waiting"),
            GameDbState::Active => write!(f, "active"),
            GameDbState::Finished => write!(f, "finished"),
            GameDbState::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Game {
    pub game_id: Uuid,
    pub guild_id: Option<i64>,
    pub channel_id: i64,
    pub game_mode: GameMode,
    pub state: GameDbState,
    pub current_round: i32,
    pub max_rounds: i32,
    pub current_turn_player: Option<i64>,
    pub timer_enabled: bool,
    pub timer_duration: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// Database model for game players
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GamePlayerRecord {
    pub id: i32,
    pub game_id: Uuid,
    pub user_id: i64,
    pub team: Option<i32>,
    pub score: i32,
    pub is_bot: bool,
    pub bot_difficulty: Option<String>,
    pub joined_at: DateTime<Utc>,
    /// Turn order for this player (0-indexed, determines play sequence)
    pub turn_order: i32,
}

// =============================================================================
// Live Game State (for WebSocket broadcast and in-memory tracking)
// =============================================================================

/// Current status of an active game
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    /// Game created but not yet started
    #[default]
    WaitingToStart,
    /// Game is actively being played
    InProgress,
    /// Round is ending, showing results
    RoundEnding,
    /// Game has finished
    Finished,
}

/// Game state data transfer object from database
/// Player data is fetched separately via get_game_players() for consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Unique game identifier
    pub game_id: Uuid,
    /// The 5x5 letter grid
    pub grid: Vec<Vec<GridCell>>,
    /// Current round number (1-indexed)
    pub current_round: u8,
    /// Total number of rounds
    pub total_rounds: u8,
    /// Index into players array for current turn
    pub current_player_index: usize,
    /// Words that have been used this game
    pub used_words: HashSet<String>,
    /// Current game status
    pub status: GameStatus,
    /// When the game was created
    pub created_at: DateTime<Utc>,
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

/// Multiplier types for grid cells
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Multiplier {
    /// Double Letter - multiplies this letter's value by 2
    #[serde(rename = "DL")]
    DoubleLetter,
    /// Triple Letter - multiplies this letter's value by 3
    #[serde(rename = "TL")]
    TripleLetter,
    /// Double Word - multiplies the entire word's score by 2 (the "pink 2x")
    #[serde(rename = "DW")]
    DoubleWord,
}

/// A single cell in the 5x5 game grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    /// The letter in this cell
    pub letter: char,
    /// Base point value of the letter
    pub value: u8,
    /// Optional multiplier (DL, TL, or DW)
    pub multiplier: Option<Multiplier>,
    /// Whether this cell contains a gem (for gem collection)
    #[serde(default)]
    pub has_gem: bool,
}

// TODO: Grid type will be used when game engine is fully integrated
#[allow(dead_code)]
pub type Grid = Vec<Vec<GridCell>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_status_serialization() {
        let status = GameStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""in_progress""#);

        let deserialized: GameStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, GameStatus::InProgress);
    }

    #[test]
    fn test_game_status_all_variants() {
        let variants = vec![
            (GameStatus::WaitingToStart, r#""waiting_to_start""#),
            (GameStatus::InProgress, r#""in_progress""#),
            (GameStatus::RoundEnding, r#""round_ending""#),
            (GameStatus::Finished, r#""finished""#),
        ];

        for (status, expected_json) in variants {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected_json);

            let deserialized: GameStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_grid_cell_serialization() {
        let cell = GridCell {
            letter: 'Q',
            value: 8, // SpellCast Q value
            multiplier: Some(Multiplier::TripleLetter),
            has_gem: true,
        };

        let json = serde_json::to_string(&cell).unwrap();
        assert!(json.contains(r#""letter":"Q""#));
        assert!(json.contains(r#""value":8"#));
        assert!(json.contains(r#""TL""#));
        assert!(json.contains(r#""has_gem":true"#));

        let deserialized: GridCell = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.letter, 'Q');
        assert_eq!(deserialized.value, 8);
        assert!(matches!(
            deserialized.multiplier,
            Some(Multiplier::TripleLetter)
        ));
        assert!(deserialized.has_gem);
    }
}
