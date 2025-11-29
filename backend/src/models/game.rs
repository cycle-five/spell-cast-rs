use std::collections::{HashMap, HashSet};

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
}

// =============================================================================
// Live Game State (for WebSocket broadcast and in-memory tracking)
// =============================================================================

/// Current status of an active game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    /// Game created but not yet started
    WaitingToStart,
    /// Game is actively being played
    InProgress,
    /// Round is ending, showing results
    RoundEnding,
    /// Game has finished
    Finished,
}

impl Default for GameStatus {
    fn default() -> Self {
        Self::WaitingToStart
    }
}

/// Player information for live game state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayer {
    /// Unique identifier for the player
    pub user_id: Uuid,
    /// Display name
    pub username: String,
    /// Discord avatar URL
    pub avatar_url: Option<String>,
    /// Current total score
    pub score: i32,
    /// Turn order (0-indexed)
    pub turn_order: u8,
    /// Whether the player is currently connected
    pub is_connected: bool,
}

impl GamePlayer {
    pub fn new(
        user_id: Uuid,
        username: String,
        avatar_url: Option<String>,
        turn_order: u8,
    ) -> Self {
        Self {
            user_id,
            username,
            avatar_url,
            score: 0,
            turn_order,
            is_connected: true,
        }
    }
}

/// Comprehensive game state for WebSocket broadcast and in-memory tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Unique game identifier
    pub game_id: Uuid,
    /// The 5x5 letter grid
    pub grid: Vec<Vec<GridCell>>,
    /// All players in the game
    pub players: Vec<GamePlayer>,
    /// Current round number (1-indexed)
    pub current_round: u8,
    /// Total number of rounds
    pub total_rounds: u8,
    /// Index into players array for current turn
    pub current_player_index: usize,
    /// Words that have been used this game
    pub used_words: HashSet<String>,
    /// Tracks which players have submitted this round (player_id -> has_submitted)
    pub round_submissions: HashMap<Uuid, bool>,
    /// Current game status
    pub status: GameStatus,
    /// When the game was created
    pub created_at: DateTime<Utc>,
}

impl GameState {
    /// Create a new game state with the given parameters
    pub fn new(
        game_id: Uuid,
        grid: Vec<Vec<GridCell>>,
        players: Vec<GamePlayer>,
        total_rounds: u8,
    ) -> Self {
        let round_submissions = players.iter().map(|p| (p.user_id, false)).collect();

        Self {
            game_id,
            grid,
            players,
            current_round: 1,
            total_rounds,
            current_player_index: 0,
            used_words: HashSet::new(),
            round_submissions,
            status: GameStatus::WaitingToStart,
            created_at: Utc::now(),
        }
    }

    /// Get the current player whose turn it is
    pub fn current_player(&self) -> Option<&GamePlayer> {
        self.players.get(self.current_player_index)
    }

    /// Get a mutable reference to the current player
    pub fn current_player_mut(&mut self) -> Option<&mut GamePlayer> {
        self.players.get_mut(self.current_player_index)
    }

    /// Check if it's the specified player's turn
    pub fn is_player_turn(&self, player_id: Uuid) -> bool {
        self.current_player()
            .map(|p| p.user_id == player_id)
            .unwrap_or(false)
    }

    /// Get a player by their user ID
    pub fn get_player(&self, user_id: Uuid) -> Option<&GamePlayer> {
        self.players.iter().find(|p| p.user_id == user_id)
    }

    /// Get a mutable reference to a player by their user ID
    pub fn get_player_mut(&mut self, user_id: Uuid) -> Option<&mut GamePlayer> {
        self.players.iter_mut().find(|p| p.user_id == user_id)
    }

    /// Check if a word has already been used
    pub fn is_word_used(&self, word: &str) -> bool {
        self.used_words.contains(&word.to_lowercase())
    }

    /// Mark a word as used
    pub fn mark_word_used(&mut self, word: &str) {
        self.used_words.insert(word.to_lowercase());
    }

    /// Check if all connected players have submitted this round
    pub fn is_round_complete(&self) -> bool {
        self.players.iter().filter(|p| p.is_connected).all(|p| {
            self.round_submissions
                .get(&p.user_id)
                .copied()
                .unwrap_or(false)
        })
    }

    /// Mark a player as having submitted for this round
    pub fn mark_player_submitted(&mut self, player_id: Uuid) {
        self.round_submissions.insert(player_id, true);
    }

    /// Reset round submissions for a new round
    pub fn reset_round_submissions(&mut self) {
        for submitted in self.round_submissions.values_mut() {
            *submitted = false;
        }
    }

    /// Check if the game is finished (all rounds complete)
    pub fn is_game_finished(&self) -> bool {
        self.current_round > self.total_rounds || self.status == GameStatus::Finished
    }

    /// Get the number of connected players
    pub fn connected_player_count(&self) -> usize {
        self.players.iter().filter(|p| p.is_connected).count()
    }
}

// Legacy type alias for backwards compatibility
pub type GamePersistenceState = GameDbState;

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

    fn create_test_grid() -> Vec<Vec<GridCell>> {
        vec![
            vec![
                GridCell {
                    letter: 'A',
                    value: 1,
                    multiplier: None,
                    has_gem: false,
                },
                GridCell {
                    letter: 'B',
                    value: 4,
                    multiplier: Some(Multiplier::DoubleLetter),
                    has_gem: false,
                },
            ],
            vec![
                GridCell {
                    letter: 'C',
                    value: 5,
                    multiplier: None,
                    has_gem: true,
                },
                GridCell {
                    letter: 'D',
                    value: 3,
                    multiplier: Some(Multiplier::TripleLetter),
                    has_gem: false,
                },
            ],
        ]
    }

    fn create_test_players() -> Vec<GamePlayer> {
        vec![
            GamePlayer::new(
                Uuid::new_v4(),
                "Player1".to_string(),
                Some("https://cdn.discord.com/avatar1.png".to_string()),
                0,
            ),
            GamePlayer::new(Uuid::new_v4(), "Player2".to_string(), None, 1),
        ]
    }

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
    fn test_game_player_serialization() {
        let player = GamePlayer::new(
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            "TestPlayer".to_string(),
            Some("https://example.com/avatar.png".to_string()),
            0,
        );

        let json = serde_json::to_string(&player).unwrap();
        assert!(json.contains("TestPlayer"));
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));

        let deserialized: GamePlayer = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, "TestPlayer");
        assert_eq!(deserialized.turn_order, 0);
        assert_eq!(deserialized.score, 0);
        assert!(deserialized.is_connected);
    }

    #[test]
    fn test_game_player_default_values() {
        let player = GamePlayer::new(Uuid::new_v4(), "NewPlayer".to_string(), None, 2);

        assert_eq!(player.score, 0);
        assert!(player.is_connected);
        assert!(player.avatar_url.is_none());
    }

    #[test]
    fn test_game_state_serialization() {
        let grid = create_test_grid();
        let players = create_test_players();
        let game_id = Uuid::new_v4();

        let game_state = GameState::new(game_id, grid, players, 5);

        let json = serde_json::to_string(&game_state).unwrap();
        assert!(json.contains(&game_id.to_string()));
        assert!(json.contains("Player1"));
        assert!(json.contains("Player2"));

        let deserialized: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.game_id, game_id);
        assert_eq!(deserialized.players.len(), 2);
        assert_eq!(deserialized.current_round, 1);
        assert_eq!(deserialized.total_rounds, 5);
        assert_eq!(deserialized.status, GameStatus::WaitingToStart);
    }

    #[test]
    fn test_game_state_current_player() {
        let grid = create_test_grid();
        let players = create_test_players();
        let player1_id = players[0].user_id;
        let player2_id = players[1].user_id;

        let game_state = GameState::new(Uuid::new_v4(), grid, players, 5);

        // First player should be current
        assert!(game_state.is_player_turn(player1_id));
        assert!(!game_state.is_player_turn(player2_id));

        let current = game_state.current_player().unwrap();
        assert_eq!(current.user_id, player1_id);
    }

    #[test]
    fn test_game_state_word_tracking() {
        let grid = create_test_grid();
        let players = create_test_players();
        let mut game_state = GameState::new(Uuid::new_v4(), grid, players, 5);

        assert!(!game_state.is_word_used("test"));

        game_state.mark_word_used("TEST");
        assert!(game_state.is_word_used("test"));
        assert!(game_state.is_word_used("TEST"));
        assert!(game_state.is_word_used("Test"));
    }

    #[test]
    fn test_game_state_round_submissions() {
        let grid = create_test_grid();
        let players = create_test_players();
        let player1_id = players[0].user_id;
        let player2_id = players[1].user_id;

        let mut game_state = GameState::new(Uuid::new_v4(), grid, players, 5);

        // Initially no one has submitted
        assert!(!game_state.is_round_complete());

        // Player 1 submits
        game_state.mark_player_submitted(player1_id);
        assert!(!game_state.is_round_complete());

        // Player 2 submits
        game_state.mark_player_submitted(player2_id);
        assert!(game_state.is_round_complete());

        // Reset for new round
        game_state.reset_round_submissions();
        assert!(!game_state.is_round_complete());
    }

    #[test]
    fn test_game_state_get_player() {
        let grid = create_test_grid();
        let players = create_test_players();
        let player1_id = players[0].user_id;

        let game_state = GameState::new(Uuid::new_v4(), grid, players, 5);

        let player = game_state.get_player(player1_id);
        assert!(player.is_some());
        assert_eq!(player.unwrap().username, "Player1");

        let non_existent = game_state.get_player(Uuid::new_v4());
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_game_state_connected_count() {
        let grid = create_test_grid();
        let mut players = create_test_players();
        players[1].is_connected = false;

        let game_state = GameState::new(Uuid::new_v4(), grid, players, 5);

        assert_eq!(game_state.connected_player_count(), 1);
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
