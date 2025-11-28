use crate::models::{GameMode, GridCell, Position};
use serde::{Deserialize, Serialize};

/// Type of lobby
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LobbyType {
    /// Lobby tied to a specific Discord channel
    Channel,
    /// Custom lobby with a shareable code, independent of Discord context
    Custom,
}

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Join a channel-based lobby (default Discord activity behavior)
    JoinChannelLobby {
        channel_id: String,
        /// Guild ID is optional for DM-based activities
        guild_id: Option<String>,
    },
    /// Create a new custom lobby with a shareable code
    CreateCustomLobby,
    /// Join an existing custom lobby by its code
    JoinCustomLobby {
        lobby_code: String,
    },
    /// Leave the current lobby
    LeaveLobby,
    CreateGame {
        mode: GameMode,
    },
    JoinGame {
        game_id: String,
    },
    LeaveGame,
    StartGame,
    SubmitWord {
        word: String,
        positions: Vec<Position>,
    },
    PassTurn,
    EnableTimer,
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Confirmation that user has joined a lobby
    LobbyJoined {
        lobby_id: String,
        lobby_type: LobbyType,
        /// For custom lobbies, the shareable code
        lobby_code: Option<String>,
    },
    /// Response to CreateCustomLobby - provides the lobby code to share
    LobbyCreated {
        lobby_code: String,
    },
    /// Sent to all connected clients when the lobby player list changes
    LobbyPlayerList {
        players: Vec<LobbyPlayerInfo>,
        /// For custom lobbies, include the code so UI can display it
        lobby_code: Option<String>,
    },
    GameCreated {
        game_id: String,
    },
    GameState {
        game_id: String,
        mode: GameMode,
        round: i32,
        max_rounds: i32,
        grid: Vec<Vec<GridCell>>,
        players: Vec<PlayerInfo>,
        current_turn: Option<i64>,
        used_words: Vec<String>,
        timer_enabled: bool,
        time_remaining: Option<u32>,
    },
    PlayerJoined {
        player: PlayerInfo,
    },
    PlayerLeft {
        user_id: i64,
    },
    GameStarted,
    TurnUpdate {
        current_player: i64,
        time_remaining: Option<u32>,
    },
    WordScored {
        word: String,
        score: i32,
        player: PlayerInfo,
        positions: Vec<Position>,
    },
    InvalidWord {
        reason: String,
    },
    RoundEnd {
        scores: Vec<ScoreInfo>,
        next_round: i32,
    },
    GameOver {
        winner: Option<i64>,
        final_scores: Vec<ScoreInfo>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub score: i32,
    pub team: Option<i32>,
}

/// Simplified player info for lobby display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyPlayerInfo {
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreInfo {
    pub user_id: i64,
    pub username: String,
    pub score: i32,
}
