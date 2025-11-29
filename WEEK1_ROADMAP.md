# Week 1: Playable Multiplayer MVP - Detailed Roadmap

> **Goal**: 2-6 players can join a lobby, play a complete 5-round game, submit words, and see a winner.

---

## Day 1: Game Session Lifecycle

### Task 1.1: Implement GameState Struct
**File**: `backend/src/models/game.rs`

**Description**: Create a comprehensive `GameState` struct that tracks all in-game state and can be serialized for WebSocket broadcast.

**Implementation Details**:
```rust
pub struct GameState {
    pub game_id: Uuid,
    pub grid: Vec<Vec<GridCell>>,
    pub players: Vec<GamePlayer>,
    pub current_round: u8,
    pub total_rounds: u8,
    pub current_player_index: usize,
    pub used_words: HashSet<String>,
    pub round_submissions: HashMap<Uuid, bool>, // player_id -> has_submitted
    pub status: GameStatus,
    pub created_at: DateTime<Utc>,
}

pub struct GamePlayer {
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub score: i32,
    pub turn_order: u8,
    pub is_connected: bool,
}

pub enum GameStatus {
    WaitingToStart,
    InProgress,
    RoundEnding,
    Finished,
}
```

**Acceptance Criteria**:
- [ ] `GameState` struct defined with all fields
- [ ] `GamePlayer` struct defined
- [ ] `GameStatus` enum defined
- [ ] Serde serialization derives added
- [ ] Unit tests for serialization

---

### Task 1.2: Create Game Database Queries
**File**: `backend/src/db/queries.rs`

**Description**: Implement database functions to create and manage game sessions.

**Functions to Implement**:
```rust
// Create a new game session
pub async fn create_game_session(
    pool: &PgPool,
    lobby_id: &str,
    created_by: Uuid,
    total_rounds: u8,
) -> Result<Uuid, sqlx::Error>

// Add players to a game
pub async fn add_game_players(
    pool: &PgPool,
    game_id: Uuid,
    players: Vec<(Uuid, u8)>, // (user_id, turn_order)
) -> Result<(), sqlx::Error>

// Create initial game board
pub async fn create_game_board(
    pool: &PgPool,
    game_id: Uuid,
    grid_json: serde_json::Value,
) -> Result<(), sqlx::Error>

// Get active game for lobby
pub async fn get_active_game_for_lobby(
    pool: &PgPool,
    lobby_id: &str,
) -> Result<Option<GameState>, sqlx::Error>
```

**Acceptance Criteria**:
- [ ] All four functions implemented
- [ ] Proper error handling with Result types
- [ ] SQL queries use parameterized statements
- [ ] Integration tests with test database

---

### Task 1.3: Implement StartGame WebSocket Handler
**File**: `backend/src/websocket/handler.rs`

**Description**: Handle the `StartGame` client message to initialize a new game session.

**Flow**:
1. Receive `StartGame` message from lobby host
2. Validate sender is lobby host
3. Validate minimum 2 players in lobby
4. Generate 5x5 grid with multipliers (use existing `grid.rs`)
5. Randomize player turn order
6. Create game session in database
7. Create `GameState` instance
8. Store in lobby's active game
9. Broadcast `GameStarted` to all players

**Message Definitions** (add to `messages.rs`):
```rust
// Client -> Server
pub enum ClientMessage {
    // ... existing
    StartGame,
}

// Server -> Client
pub enum ServerMessage {
    // ... existing
    GameStarted {
        game_id: String,
        grid: Vec<Vec<GridCell>>,
        players: Vec<GamePlayerInfo>,
        current_player_id: String,
        total_rounds: u8,
    },
    GameError {
        code: String,
        message: String,
    },
}
```

**Acceptance Criteria**:
- [ ] `StartGame` message type added
- [ ] `GameStarted` message type added
- [ ] Handler validates host permission
- [ ] Handler validates player count (2-6)
- [ ] Grid generated using existing `grid.rs` module
- [ ] Player order randomized
- [ ] Database records created
- [ ] All players receive `GameStarted` broadcast
- [ ] Error returned if game already in progress

---

### Task 1.4: Add GameStarted Frontend Handler
**File**: `frontend/js/websocket.js`, `frontend/js/game-ui.js`

**Description**: Handle the `GameStarted` message and transition UI from lobby to game screen.

**Implementation**:
```javascript
// In websocket.js - add to message handler
case 'GameStarted':
    this.onGameStarted(data);
    break;

// Callback to game-ui.js
onGameStarted(data) {
    gameUI.initializeGame({
        gameId: data.game_id,
        grid: data.grid,
        players: data.players,
        currentPlayerId: data.current_player_id,
        totalRounds: data.total_rounds
    });
}
```

**UI Changes**:
- Hide lobby screen (`#lobby-screen`)
- Show game screen (`#game-screen`)
- Render grid with letters and multipliers
- Show player list with scores (all 0)
- Highlight current player's turn
- Show round indicator (Round 1/5)

**Acceptance Criteria**:
- [ ] `GameStarted` message parsed correctly
- [ ] Screen transition from lobby to game
- [ ] Grid renders with all letters visible
- [ ] Multipliers shown (2x, 3x indicators)
- [ ] Player list displays with turn indicator
- [ ] Round counter shows "Round 1 of 5"

---

### Task 1.5: Add Start Game Button to Lobby UI
**File**: `frontend/js/game-ui.js`, `frontend/index.html`

**Description**: Add a "Start Game" button visible only to the lobby host.

**HTML**:
```html
<button id="start-game-btn" class="btn-primary" style="display: none;">
    Start Game
</button>
```

**JavaScript**:
```javascript
// Show button only for host when 2+ players
updateStartButton() {
    const btn = document.getElementById('start-game-btn');
    const isHost = this.currentUserId === this.lobby.hostId;
    const hasEnoughPlayers = this.lobby.players.length >= 2;
    btn.style.display = (isHost && hasEnoughPlayers) ? 'block' : 'none';
}

// Send StartGame on click
document.getElementById('start-game-btn').addEventListener('click', () => {
    gameClient.send({ type: 'StartGame' });
});
```

**Acceptance Criteria**:
- [ ] Button exists in lobby screen HTML
- [ ] Button only visible to host
- [ ] Button only enabled with 2+ players
- [ ] Clicking sends `StartGame` message
- [ ] Button shows loading state while waiting

---

## Day 2: Turn System & Word Submission

### Task 2.1: Implement Turn Rotation Logic
**File**: `backend/src/game/turns.rs` (new file)

**Description**: Create a module to manage turn order and rotation within a game.

**Functions**:
```rust
pub struct TurnManager;

impl TurnManager {
    /// Get the current player's user ID
    pub fn get_current_player(game_state: &GameState) -> Uuid

    /// Check if it's the specified player's turn
    pub fn is_player_turn(game_state: &GameState, player_id: Uuid) -> bool

    /// Advance to the next player's turn
    /// Returns (new_player_id, round_complete)
    pub fn advance_turn(game_state: &mut GameState) -> (Uuid, bool)

    /// Check if all players have submitted this round
    pub fn is_round_complete(game_state: &GameState) -> bool

    /// Start a new round (reset submissions, optionally new grid)
    pub fn start_new_round(game_state: &mut GameState, new_grid: Option<Vec<Vec<GridCell>>>)
}
```

**Logic**:
- Turn order based on `turn_order` field (0, 1, 2, ...)
- Skip disconnected players (if `is_connected == false`)
- Round complete when all connected players have submitted
- Handle edge case: only 1 player connected

**Acceptance Criteria**:
- [ ] All functions implemented
- [ ] Disconnected players are skipped
- [ ] Round completion detected correctly
- [ ] Unit tests for all turn scenarios
- [ ] Edge case: single player remaining

---

### Task 2.2: Implement SubmitWord Handler
**File**: `backend/src/websocket/handler.rs`

**Description**: Handle word submissions from players during their turn.

**Message Definition**:
```rust
// Client -> Server
SubmitWord {
    path: Vec<(u8, u8)>,  // Grid positions: [(row, col), ...]
}

// Server -> Client (broadcast to all)
WordSubmitted {
    player_id: String,
    word: String,
    path: Vec<(u8, u8)>,
    score: i32,
    new_total: i32,
}

WordRejected {
    reason: String,  // "not_your_turn", "invalid_path", "not_in_dictionary", "already_used"
}

TurnChanged {
    current_player_id: String,
    round: u8,
}
```

**Validation Pipeline**:
1. Check it's the player's turn → `WordRejected { reason: "not_your_turn" }`
2. Extract word from grid using path
3. Validate path (adjacent cells, no repeats) → `WordRejected { reason: "invalid_path" }`
4. Validate word length (≥3 chars) → `WordRejected { reason: "word_too_short" }`
5. Check dictionary → `WordRejected { reason: "not_in_dictionary" }`
6. Check not already used → `WordRejected { reason: "already_used" }`
7. Calculate score with multipliers
8. Update player score in game state
9. Add word to used words set
10. Mark player as submitted this round
11. Broadcast `WordSubmitted` to all players
12. Advance turn and broadcast `TurnChanged`

**Acceptance Criteria**:
- [ ] `SubmitWord` message type added
- [ ] `WordSubmitted` broadcast message added
- [ ] `WordRejected` error message added
- [ ] `TurnChanged` message added
- [ ] All validation steps implemented in order
- [ ] Score calculation uses existing `scorer.rs`
- [ ] Database move recorded (game_moves table)
- [ ] Turn advances after successful submission

---

### Task 2.3: Implement SkipTurn Handler
**File**: `backend/src/websocket/handler.rs`

**Description**: Allow players to skip their turn if they can't find a valid word.

**Message Definition**:
```rust
// Client -> Server
SkipTurn

// Server -> Client (broadcast)
TurnSkipped {
    player_id: String,
}
```

**Logic**:
1. Validate it's the player's turn
2. Mark player as submitted (with 0 points)
3. Broadcast `TurnSkipped`
4. Advance turn

**Acceptance Criteria**:
- [ ] `SkipTurn` message type added
- [ ] `TurnSkipped` broadcast added
- [ ] Only current player can skip
- [ ] No points awarded
- [ ] Turn advances correctly

---

### Task 2.4: Integrate Existing Validation Modules
**File**: `backend/src/websocket/handler.rs`

**Description**: Wire up the existing `validator.rs` and `scorer.rs` modules into the word submission handler.

**Integration Points**:
```rust
use crate::game::validator::{validate_path, validate_word};
use crate::game::scorer::calculate_score;
use crate::dictionary::is_valid_word;

// In SubmitWord handler:
// 1. validate_path(path, grid) - checks adjacency, no repeats
// 2. extract_word(path, grid) - get letters from positions
// 3. is_valid_word(word, dictionary) - dictionary lookup
// 4. calculate_score(word, path, grid) - score with multipliers
```

**Verify Existing Functions**:
- `validator.rs`: Confirm `validate_path` checks 8-directional adjacency
- `scorer.rs`: Confirm multipliers (2x, 3x) are applied
- `dictionary/mod.rs`: Confirm word lookup is case-insensitive

**Acceptance Criteria**:
- [ ] Path validation integrated
- [ ] Word extraction working
- [ ] Dictionary lookup integrated
- [ ] Score calculation integrated
- [ ] All edge cases handled

---

### Task 2.5: Frontend Word Selection UI
**File**: `frontend/js/game-ui.js`

**Description**: Implement click/tap interaction to select letters on the grid.

**Features**:
- Click letter to start selection
- Click adjacent letter to extend path
- Click non-adjacent shows error (shake animation)
- Click same letter twice to deselect
- Show path with lines connecting letters
- Display formed word in preview area
- "Clear" button to reset selection
- "Submit" button (disabled if invalid)

**State Management**:
```javascript
class GridSelection {
    selectedPath = [];        // Array of {row, col}
    selectedWord = '';        // Current word string

    addCell(row, col) { }
    removeLastCell() { }
    clear() { }
    isAdjacent(row, col) { }
    isSelected(row, col) { }
    getWord() { }
}
```

**Visual Feedback**:
- Selected cells: highlighted background
- Path: SVG lines connecting centers
- Valid word: green preview text
- Invalid/short word: red preview text

**Acceptance Criteria**:
- [ ] Cells are clickable
- [ ] Only adjacent cells can be selected
- [ ] Visual path drawn between cells
- [ ] Word preview updates in real-time
- [ ] Clear button works
- [ ] Cannot select same cell twice
- [ ] Works on both desktop and mobile (touch)

---

### Task 2.6: Frontend Submit Word Integration
**File**: `frontend/js/game-ui.js`, `frontend/js/websocket.js`

**Description**: Connect the submit button to send words to the server.

**Flow**:
1. User clicks "Submit Word" button
2. Validate locally: path length ≥ 3
3. Send `SubmitWord` message with path
4. Show loading state on button
5. Handle `WordSubmitted` or `WordRejected` response
6. Update UI accordingly
7. Clear selection on success
8. Show error toast on rejection

**JavaScript**:
```javascript
// Submit button handler
submitWord() {
    if (this.selectedPath.length < 3) {
        this.showError('Word must be at least 3 letters');
        return;
    }

    this.setSubmitLoading(true);
    gameClient.send({
        type: 'SubmitWord',
        path: this.selectedPath.map(c => [c.row, c.col])
    });
}

// Response handlers
onWordSubmitted(data) {
    this.clearSelection();
    this.updatePlayerScore(data.player_id, data.new_total);
    this.showWordAnimation(data.word, data.score);
    this.setSubmitLoading(false);
}

onWordRejected(data) {
    this.showError(this.getErrorMessage(data.reason));
    this.setSubmitLoading(false);
}
```

**Acceptance Criteria**:
- [ ] Submit button sends message
- [ ] Loading state shown during request
- [ ] Success clears selection
- [ ] Success shows score animation
- [ ] Rejection shows error message
- [ ] Scores update in player list

---

## Day 3: Round Management & Game Flow

### Task 3.1: Implement Round Transition Logic
**File**: `backend/src/game/turns.rs`, `backend/src/websocket/handler.rs`

**Description**: Handle the transition between rounds after all players submit.

**Flow**:
1. After last player submits, detect round complete
2. Calculate round scores
3. If round < 5: start new round
4. If round == 5: end game

**Messages**:
```rust
// Server -> Client
RoundEnded {
    round: u8,
    scores: Vec<PlayerRoundScore>,  // player_id, round_score, total_score
    next_round: Option<u8>,         // None if game over
}

pub struct PlayerRoundScore {
    player_id: String,
    words_this_round: Vec<ScoredWord>,
    round_score: i32,
    total_score: i32,
}
```

**Options** (configurable per game):
- `new_grid_each_round: bool` - Generate fresh grid or keep same
- `clear_used_words: bool` - Reset used words or persist

**Acceptance Criteria**:
- [ ] Round completion detected automatically
- [ ] `RoundEnded` message broadcasted
- [ ] Round scores calculated correctly
- [ ] New round starts with reset state
- [ ] Game ends after round 5

---

### Task 3.2: Implement Game End Logic
**File**: `backend/src/websocket/handler.rs`

**Description**: Handle game completion and determine the winner.

**Flow**:
1. After round 5 ends, calculate final standings
2. Determine winner (highest score)
3. Handle ties (multiple winners)
4. Update database with final results
5. Broadcast `GameEnded`
6. Return lobby to waiting state

**Messages**:
```rust
// Server -> Client
GameEnded {
    winner_ids: Vec<String>,  // Multiple if tie
    final_standings: Vec<FinalStanding>,
    game_duration_seconds: u64,
}

pub struct FinalStanding {
    player_id: String,
    username: String,
    total_score: i32,
    words_found: u32,
    best_word: String,
    best_word_score: i32,
    rank: u8,  // 1, 2, 3... (ties share rank)
}
```

**Database Updates**:
- Update `games` table: `status = 'finished'`, `winner_id`, `ended_at`
- Update `game_players` table: `final_score`, `final_rank`

**Acceptance Criteria**:
- [ ] Winner determined correctly
- [ ] Ties handled (multiple winners)
- [ ] `GameEnded` message includes all stats
- [ ] Database updated with final results
- [ ] Lobby returns to waiting state

---

### Task 3.3: Frontend Round End Screen
**File**: `frontend/js/game-ui.js`, `frontend/index.html`

**Description**: Show a brief round summary between rounds.

**UI Elements**:
- Modal/overlay showing round results
- Player scores for this round
- Words each player found
- "Next Round" countdown (5 seconds)
- Updated total scores

**Flow**:
1. Receive `RoundEnded` message
2. Show round summary modal
3. Display scores and words
4. Auto-dismiss after 5 seconds OR click "Continue"
5. Transition to next round

**Acceptance Criteria**:
- [ ] Round summary modal displays
- [ ] Shows round scores
- [ ] Shows words found per player
- [ ] Auto-continues after delay
- [ ] Can click to continue early

---

### Task 3.4: Frontend Game Results Screen
**File**: `frontend/js/game-ui.js`, `frontend/index.html`

**Description**: Show final game results with winner celebration.

**UI Elements**:
- Full-screen results view
- Winner announcement (with celebration effect)
- Final standings table
- Per-player stats (best word, total words)
- "Play Again" button → return to lobby
- "Leave" button → disconnect

**Standings Display**:
```
🏆 1st: PlayerOne - 450 pts
🥈 2nd: PlayerTwo - 380 pts
🥉 3rd: PlayerThree - 290 pts
```

**Acceptance Criteria**:
- [ ] Results screen shows on `GameEnded`
- [ ] Winner clearly highlighted
- [ ] All player stats displayed
- [ ] Tie handling shown correctly
- [ ] "Play Again" returns to lobby
- [ ] "Leave" disconnects cleanly

---

### Task 3.5: Frontend Turn Indicator UI
**File**: `frontend/js/game-ui.js`

**Description**: Clearly show whose turn it is and update on changes.

**UI Elements**:
- Current player highlight in player list
- "Your Turn!" banner when it's your turn
- "Waiting for [PlayerName]..." when it's not
- Turn timer (optional: 60 second limit)
- Disable submit button when not your turn

**Visual States**:
```
Your turn:
- Green "YOUR TURN" banner
- Submit button enabled
- Grid fully interactive

Other's turn:
- Gray "Waiting for Alex..." text
- Submit button disabled
- Grid shows as view-only (no selection)
```

**Acceptance Criteria**:
- [ ] Current player clearly indicated
- [ ] "Your Turn" banner shows
- [ ] Submit button disabled when not your turn
- [ ] Grid non-interactive when not your turn
- [ ] Smooth transition on turn change

---

## Day 4: Dictionary & Word Validation

### Task 4.1: Load Dictionary on Server Startup
**File**: `backend/src/dictionary/mod.rs`

**Description**: Load a comprehensive word list into memory for fast lookups.

**Implementation**:
```rust
use std::collections::HashSet;
use once_cell::sync::Lazy;

static DICTIONARY: Lazy<HashSet<String>> = Lazy::new(|| {
    load_dictionary()
});

fn load_dictionary() -> HashSet<String> {
    // Option 1: Load from embedded file
    let words = include_str!("../../assets/words.txt");

    // Option 2: Load from file path
    // let words = std::fs::read_to_string("assets/words.txt").unwrap();

    words
        .lines()
        .map(|w| w.trim().to_lowercase())
        .filter(|w| w.len() >= 3 && w.chars().all(|c| c.is_ascii_alphabetic()))
        .collect()
}

pub fn is_valid_word(word: &str) -> bool {
    DICTIONARY.contains(&word.to_lowercase())
}

pub fn dictionary_size() -> usize {
    DICTIONARY.len()
}
```

**Word List Source**:
- Use SOWPODS (Scrabble dictionary) or similar
- ~270,000 words for comprehensive coverage
- Or TWL (Tournament Word List) ~180,000 words
- Filter to 3+ letter words only

**Acceptance Criteria**:
- [ ] Dictionary loads on startup
- [ ] 50,000+ words minimum
- [ ] Lookup is O(1) with HashSet
- [ ] Case-insensitive matching
- [ ] Startup time < 2 seconds

---

### Task 4.2: Create/Obtain Word List File
**File**: `backend/assets/words.txt`

**Description**: Add a comprehensive word list file to the project.

**Sources** (choose one):
1. SOWPODS: https://www.wordgamedictionary.com/sowpods/
2. TWL06: https://www.wordgamedictionary.com/twl06/
3. Enable word list: https://www.wordgame.com/enable/

**Format**:
```
aardvark
aardvarks
aardwolf
...
zymurgy
```

**Filtering Requirements**:
- Only alphabetic characters (no hyphens, apostrophes)
- 3-15 letters length
- Lowercase normalized
- No proper nouns
- No offensive words (optional filter)

**Acceptance Criteria**:
- [ ] Word list file exists at correct path
- [ ] Contains 50,000+ valid words
- [ ] One word per line
- [ ] Lowercase format
- [ ] No invalid characters

---

### Task 4.3: Enhance Validation Error Messages
**File**: `backend/src/websocket/handler.rs`

**Description**: Provide clear, helpful error messages for word rejections.

**Error Codes & Messages**:
```rust
pub enum WordRejectionReason {
    NotYourTurn,
    PathTooShort,      // < 3 letters
    InvalidPath,        // Non-adjacent or repeated cells
    NotInDictionary,
    AlreadyUsed,
    PathOutOfBounds,   // Invalid grid coordinates
}

impl WordRejectionReason {
    pub fn message(&self) -> &'static str {
        match self {
            Self::NotYourTurn => "It's not your turn",
            Self::PathTooShort => "Word must be at least 3 letters",
            Self::InvalidPath => "Letters must be adjacent with no repeats",
            Self::NotInDictionary => "Word not found in dictionary",
            Self::AlreadyUsed => "This word has already been played",
            Self::PathOutOfBounds => "Invalid grid position",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotYourTurn => "not_your_turn",
            Self::PathTooShort => "path_too_short",
            Self::InvalidPath => "invalid_path",
            Self::NotInDictionary => "not_in_dictionary",
            Self::AlreadyUsed => "already_used",
            Self::PathOutOfBounds => "path_out_of_bounds",
        }
    }
}
```

**Acceptance Criteria**:
- [ ] All rejection reasons have codes
- [ ] All rejection reasons have user-friendly messages
- [ ] Correct reason returned for each failure case
- [ ] Frontend displays appropriate message

---

### Task 4.4: Frontend Error Display
**File**: `frontend/js/game-ui.js`

**Description**: Show word rejection errors clearly to the user.

**UI Component**:
```javascript
class ErrorToast {
    show(message, duration = 3000) {
        const toast = document.getElementById('error-toast');
        toast.textContent = message;
        toast.classList.add('visible');

        setTimeout(() => {
            toast.classList.remove('visible');
        }, duration);
    }
}
```

**Error Display Behavior**:
- Red toast notification slides in from top
- Shows for 3 seconds then fades
- Shake animation on the grid (optional)
- Don't clear user's selection on rejection (let them modify)

**CSS**:
```css
.error-toast {
    position: fixed;
    top: 20px;
    left: 50%;
    transform: translateX(-50%) translateY(-100%);
    background: #ff4444;
    color: white;
    padding: 12px 24px;
    border-radius: 8px;
    transition: transform 0.3s ease;
}

.error-toast.visible {
    transform: translateX(-50%) translateY(0);
}
```

**Acceptance Criteria**:
- [ ] Toast component implemented
- [ ] Shows rejection message clearly
- [ ] Auto-dismisses after delay
- [ ] Doesn't block gameplay
- [ ] Selection preserved on error

---

## Day 5: Reconnection & Player Management

### Task 5.1: Handle Player Reconnection to Game
**File**: `backend/src/websocket/handler.rs`

**Description**: Allow disconnected players to rejoin an in-progress game.

**Flow**:
1. Player reconnects via WebSocket
2. Check if player has an active game
3. If yes, send current game state
4. Mark player as connected
5. Broadcast reconnection to other players

**Messages**:
```rust
// Server -> Client (to reconnecting player)
GameStateSync {
    game_id: String,
    grid: Vec<Vec<GridCell>>,
    players: Vec<GamePlayerInfo>,
    current_player_id: String,
    current_round: u8,
    used_words: Vec<String>,
    your_score: i32,
    has_submitted_this_round: bool,
}

// Server -> Client (to other players)
PlayerReconnected {
    player_id: String,
    username: String,
}
```

**Acceptance Criteria**:
- [ ] Reconnecting player receives full game state
- [ ] Player can continue playing from correct position
- [ ] Other players notified of reconnection
- [ ] If it was their turn, they can still submit
- [ ] Score and submission state preserved

---

### Task 5.2: Handle Player Disconnect During Game
**File**: `backend/src/websocket/handler.rs`, `backend/src/main.rs`

**Description**: Gracefully handle players disconnecting mid-game.

**Behavior**:
1. Mark player as disconnected (not removed)
2. Start grace period timer (60 seconds)
3. If their turn, auto-skip after 30 seconds
4. If they don't reconnect, remove from game
5. Game continues with remaining players

**Messages**:
```rust
// Server -> Client
PlayerDisconnected {
    player_id: String,
    username: String,
    grace_period_seconds: u32,
}

PlayerTimedOut {
    player_id: String,
    username: String,
    reason: String,  // "turn_timeout" or "disconnect_timeout"
}
```

**Turn Skip Logic**:
- If disconnected player's turn, wait 30 seconds
- Then auto-skip with 0 points
- Broadcast `TurnSkipped` with timeout reason

**Acceptance Criteria**:
- [ ] Disconnected player marked, not removed
- [ ] Other players notified of disconnect
- [ ] Turn auto-skips after timeout
- [ ] Player removed after grace period
- [ ] Game ends if < 2 players remain

---

### Task 5.3: Handle Host Leaving
**File**: `backend/src/main.rs`

**Description**: Transfer host privileges when the host leaves.

**Logic**:
1. When host disconnects/leaves lobby
2. Find next player by join order
3. Assign them as new host
4. Broadcast host change to all players

**Messages**:
```rust
// Server -> Client
HostChanged {
    new_host_id: String,
    new_host_username: String,
}
```

**Edge Cases**:
- Last player becomes host (alone in lobby)
- Host leaves during game (game continues, no host needed)
- All players leave (cleanup lobby/game)

**Acceptance Criteria**:
- [ ] New host assigned automatically
- [ ] New host can start games
- [ ] Broadcast notifies all players
- [ ] Works in lobby and in-game states

---

### Task 5.4: Frontend Reconnection UI
**File**: `frontend/js/websocket.js`, `frontend/js/game-ui.js`

**Description**: Handle reconnection gracefully on the frontend.

**Reconnection Flow**:
1. WebSocket disconnects (already has exponential backoff)
2. Show "Reconnecting..." overlay
3. On reconnect, authenticate again
4. Receive `GameStateSync` if game in progress
5. Restore game UI from sync data
6. Hide overlay, resume play

**UI Elements**:
```html
<div id="reconnecting-overlay" class="overlay hidden">
    <div class="overlay-content">
        <div class="spinner"></div>
        <p>Reconnecting...</p>
        <p class="attempt-count">Attempt 1 of 5</p>
    </div>
</div>
```

**Acceptance Criteria**:
- [ ] Reconnection overlay shows during disconnect
- [ ] Attempt counter updates
- [ ] Game state restored on reconnect
- [ ] Seamless return to gameplay
- [ ] Error shown if max attempts exceeded

---

### Task 5.5: Game Cleanup on Empty
**File**: `backend/src/main.rs`

**Description**: Clean up game state when all players leave.

**Cleanup Logic**:
1. When last player disconnects from game
2. Wait grace period (120 seconds)
3. If no one returns, mark game as abandoned
4. Clean up in-memory state
5. Update database status

**Database Update**:
```sql
UPDATE games
SET status = 'abandoned', ended_at = NOW()
WHERE id = $1;
```

**Acceptance Criteria**:
- [ ] Empty games cleaned up after timeout
- [ ] Database records preserved (for stats)
- [ ] In-memory state freed
- [ ] No orphaned game sessions

---

## Day 6: Testing & Bug Fixes

### Task 6.1: End-to-End Game Flow Tests
**File**: `backend/tests/game_flow_test.rs` (new)

**Description**: Write integration tests for complete game scenarios.

**Test Scenarios**:
```rust
#[tokio::test]
async fn test_two_player_game_complete() {
    // 1. Two players join lobby
    // 2. Host starts game
    // 3. Each player submits word per round
    // 4. Game completes after 5 rounds
    // 5. Winner determined correctly
}

#[tokio::test]
async fn test_player_disconnect_during_game() {
    // 1. Start 3-player game
    // 2. One player disconnects
    // 3. Turn skips after timeout
    // 4. Game continues with 2 players
}

#[tokio::test]
async fn test_invalid_word_rejection() {
    // 1. Start game
    // 2. Submit non-dictionary word
    // 3. Verify rejection
    // 4. Player can retry
}

#[tokio::test]
async fn test_player_reconnection() {
    // 1. Start game
    // 2. Player disconnects
    // 3. Player reconnects
    // 4. Game state synced correctly
}
```

**Acceptance Criteria**:
- [ ] All test scenarios pass
- [ ] Tests run in CI/CD
- [ ] No race conditions
- [ ] Cleanup after each test

---

### Task 6.2: WebSocket Message Validation Tests
**File**: `backend/tests/websocket_test.rs` (new)

**Description**: Test all WebSocket message types are handled correctly.

**Test Cases**:
```rust
#[test]
fn test_start_game_validation() {
    // Only host can start
    // Need 2+ players
    // Can't start if game in progress
}

#[test]
fn test_submit_word_validation() {
    // Path must be adjacent
    // Word must be in dictionary
    // Word can't be reused
    // Must be player's turn
}

#[test]
fn test_message_serialization() {
    // All message types serialize/deserialize correctly
    // Invalid messages rejected gracefully
}
```

**Acceptance Criteria**:
- [ ] All message types have tests
- [ ] Edge cases covered
- [ ] Serialization round-trips work

---

### Task 6.3: Manual Testing Checklist
**File**: `docs/TESTING_CHECKLIST.md` (new)

**Description**: Create a manual testing checklist for QA.

**Checklist Items**:
```markdown
## Lobby
- [ ] Join lobby via channel
- [ ] Join via custom code
- [ ] See other players join
- [ ] Host can start with 2+ players
- [ ] Non-host cannot start

## Gameplay
- [ ] Grid renders correctly
- [ ] Can select adjacent letters
- [ ] Cannot select non-adjacent
- [ ] Word preview shows
- [ ] Submit works on your turn
- [ ] Cannot submit on other's turn
- [ ] Score updates after submission
- [ ] Turn advances after submission

## Words
- [ ] Valid words accepted
- [ ] Invalid words rejected
- [ ] Short words rejected (<3 letters)
- [ ] Already-used words rejected
- [ ] Error messages display

## Rounds & Game End
- [ ] Round ends after all submit
- [ ] Round summary shows
- [ ] New round starts correctly
- [ ] Game ends after round 5
- [ ] Winner shown correctly
- [ ] Ties handled

## Edge Cases
- [ ] Disconnect and reconnect
- [ ] Host leaves (new host assigned)
- [ ] Skip turn works
- [ ] All players leave (game cleaned up)
```

**Acceptance Criteria**:
- [ ] Checklist covers all features
- [ ] Can complete full checklist
- [ ] All items pass

---

## Day 7: Polish & Final Fixes

### Task 7.1: Score Animation
**File**: `frontend/js/game-ui.js`, `frontend/styles/grid.css`

**Description**: Add visual feedback when words are scored.

**Animation**:
- Word floats up from grid
- Score appears (+45 points)
- Fades out after 1.5 seconds
- Different colors for different score ranges

**CSS**:
```css
.score-popup {
    position: absolute;
    font-size: 24px;
    font-weight: bold;
    animation: scoreFloat 1.5s ease-out forwards;
}

@keyframes scoreFloat {
    0% { opacity: 1; transform: translateY(0); }
    100% { opacity: 0; transform: translateY(-50px); }
}

.score-low { color: #888; }     /* < 10 */
.score-medium { color: #4CAF50; } /* 10-30 */
.score-high { color: #FF9800; }   /* 30-50 */
.score-epic { color: #9C27B0; }   /* 50+ */
```

**Acceptance Criteria**:
- [ ] Animation plays on word submission
- [ ] Score visible during animation
- [ ] Color indicates score quality
- [ ] Doesn't block gameplay

---

### Task 7.2: Sound Effects (Optional)
**File**: `frontend/js/audio.js` (new)

**Description**: Add basic sound effects for game events.

**Sounds**:
- `word-submit.mp3` - Successfully submitted word
- `word-reject.mp3` - Word rejected
- `turn-change.mp3` - Turn changed to you
- `game-win.mp3` - You won the game
- `round-end.mp3` - Round completed

**Implementation**:
```javascript
class AudioManager {
    constructor() {
        this.sounds = {
            submit: new Audio('/sounds/word-submit.mp3'),
            reject: new Audio('/sounds/word-reject.mp3'),
            yourTurn: new Audio('/sounds/turn-change.mp3'),
            win: new Audio('/sounds/game-win.mp3'),
            roundEnd: new Audio('/sounds/round-end.mp3'),
        };
        this.enabled = true;
    }

    play(soundName) {
        if (this.enabled && this.sounds[soundName]) {
            this.sounds[soundName].play();
        }
    }

    toggle() {
        this.enabled = !this.enabled;
    }
}
```

**Acceptance Criteria**:
- [ ] Sound files exist in assets
- [ ] Sounds play at correct times
- [ ] Mute toggle available
- [ ] Sounds don't overlap badly

---

### Task 7.3: Loading States
**File**: `frontend/js/game-ui.js`

**Description**: Add loading indicators for all async operations.

**Loading States Needed**:
- Starting game (after click Start)
- Submitting word (after click Submit)
- Reconnecting (during WebSocket reconnect)
- Loading lobby (when first connecting)

**Implementation**:
```javascript
setButtonLoading(buttonId, loading) {
    const btn = document.getElementById(buttonId);
    btn.disabled = loading;
    btn.textContent = loading ? 'Loading...' : btn.dataset.originalText;
}
```

**Acceptance Criteria**:
- [ ] All async actions show loading
- [ ] Buttons disabled during loading
- [ ] Loading text/spinners visible
- [ ] No double-submissions possible

---

### Task 7.4: Error Boundary & Crash Recovery
**File**: `frontend/js/main.js`

**Description**: Handle unexpected errors gracefully.

**Error Handling**:
```javascript
window.onerror = function(msg, url, lineNo, columnNo, error) {
    console.error('Uncaught error:', error);
    showErrorScreen('Something went wrong. Please refresh to continue.');
    return true;
};

window.onunhandledrejection = function(event) {
    console.error('Unhandled promise rejection:', event.reason);
    showErrorScreen('Connection error. Please refresh to reconnect.');
};
```

**Recovery Options**:
- "Refresh" button on error screen
- Preserve game ID in localStorage
- Attempt reconnect on refresh

**Acceptance Criteria**:
- [ ] Errors don't crash silently
- [ ] Error screen shows helpful message
- [ ] User can recover by refreshing
- [ ] Error details logged for debugging

---

### Task 7.5: Final Integration Verification
**File**: N/A (testing task)

**Description**: Verify complete game flow works with multiple real users.

**Testing Scenarios**:
1. **Happy Path**: 4 players complete full game
2. **Reconnect Test**: Player refreshes mid-game
3. **Leave Test**: Player closes tab, others continue
4. **Edge Cases**: Ties, skip turns, all players same word

**Performance Checks**:
- Message latency < 100ms
- No memory leaks over multiple games
- Grid renders smoothly
- Score updates instantly

**Acceptance Criteria**:
- [ ] All scenarios work correctly
- [ ] No crashes during testing
- [ ] Performance acceptable
- [ ] Ready for broader testing

---

# Week 1 Summary

## Total Tasks: 28

| Day | Tasks | Focus |
|-----|-------|-------|
| 1 | 5 | Game session lifecycle |
| 2 | 6 | Turn system & word submission |
| 3 | 5 | Round management & UI |
| 4 | 4 | Dictionary & validation |
| 5 | 5 | Reconnection & edge cases |
| 6 | 3 | Testing |
| 7 | 5 | Polish & final fixes |

## Dependencies
```
Day 1 → Day 2 (need game state for turns)
Day 2 → Day 3 (need submission for rounds)
Day 4 → Day 2 (dictionary needed for validation)
Day 5 → Day 1,2,3 (need working game for reconnection)
Day 6 → Day 1-5 (testing requires features)
Day 7 → Day 1-6 (polish requires working game)
```

## MVP Definition
At the end of Week 1, the game should support:
- ✅ 2-6 players joining via Discord Activity
- ✅ Lobby with host controls
- ✅ 5-round game with turn-based play
- ✅ Word submission with dictionary validation
- ✅ Real-time score updates
- ✅ Round and game end screens
- ✅ Basic reconnection handling
- ✅ Win/lose determination

---

*Last Updated: 2025-11-29*
