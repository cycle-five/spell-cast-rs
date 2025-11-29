# Spell Cast Redux - Game Development Roadmap

> **Goal**: A fully playable multiplayer word game as a Discord Activity

## Current State Summary

The project has solid foundations in place:
- **Complete**: Discord OAuth2 authentication, JWT tokens, WebSocket infrastructure
- **Complete**: Lobby system (channel-based + custom codes), player connection tracking
- **Complete**: Game grid generation, letter distribution, cell multipliers
- **Complete**: Word validation logic, path validation, scoring system
- **Complete**: Database schema, migrations, basic queries
- **Incomplete**: Turn-based game flow, word submission pipeline, real-time game sync

---

# Week 1: Playable Multiplayer MVP

**Primary Goal**: 2-6 players can join a lobby, play a complete game with turns, submit words, and see a winner.

## Day 1: Game Session Lifecycle

### Tasks
1. **Implement `StartGame` handler** (`backend/src/websocket/handler.rs`)
   - Create game session in database
   - Generate initial 5x5 grid with multipliers
   - Assign player order randomly
   - Transition lobby state to "in_game"
   - Broadcast `GameStarted` to all players with grid data

2. **Implement `GameState` struct** (`backend/src/models/game.rs`)
   - Track current round (1-5)
   - Track current player turn
   - Track used words per game
   - Track player scores
   - Serialize for WebSocket broadcast

3. **Database: Game creation queries** (`backend/src/db/queries.rs`)
   - `create_game_session()` - Insert into games table
   - `add_game_players()` - Link players to game
   - `create_game_board()` - Store initial grid

### Acceptance Criteria
- [ ] Host can click "Start Game" when 2+ players in lobby
- [ ] All players receive grid and game state
- [ ] Database records game session

---

## Day 2: Turn System & Word Submission

### Tasks
1. **Implement turn rotation** (`backend/src/main.rs`)
   - `get_current_player()` - Who's turn is it?
   - `advance_turn()` - Move to next player
   - `is_player_turn()` - Validate move authority
   - Handle round advancement after all players submit

2. **Word submission pipeline** (`backend/src/websocket/handler.rs`)
   - Receive `SubmitWord { path: Vec<(u8, u8)> }`
   - Validate it's the player's turn
   - Extract word from grid using path
   - Validate word exists in dictionary
   - Validate path is valid (adjacent, no repeats)
   - Check word not already used this game
   - Calculate score with multipliers
   - Store move in database
   - Broadcast result to all players

3. **Skip turn option**
   - Player can pass if no valid words found
   - Award 0 points, advance turn

### Acceptance Criteria
- [ ] Only current player can submit words
- [ ] Invalid words are rejected with error message
- [ ] Valid words update score and advance turn
- [ ] All players see word submission results in real-time

---

## Day 3: Frontend Game Board Interaction

### Tasks
1. **Grid rendering** (`frontend/js/game-ui.js`)
   - Render 5x5 grid from server data
   - Show letter values and multipliers (2x, 3x indicators)
   - Highlight current player's turn

2. **Letter selection** (`frontend/js/game-ui.js`)
   - Click/tap to select letters
   - Visual path drawing between selected letters
   - Display formed word preview
   - Validate adjacency on client side
   - Clear selection button

3. **Submit word button**
   - Send `SubmitWord` message via WebSocket
   - Disable during opponent turns
   - Show loading state while processing

4. **Real-time updates**
   - Handle `TurnChanged` messages
   - Handle `WordScored` messages with animation
   - Update scoreboard in real-time

### Acceptance Criteria
- [ ] Players can visually select letters on grid
- [ ] Path shows valid/invalid status
- [ ] Submit button works and shows feedback
- [ ] Scores update in real-time for all players

---

## Day 4: Round Management & Game End

### Tasks
1. **Round progression** (`backend/src/main.rs`)
   - After all players submit once → new round
   - Generate new grid for each round (or keep same, configurable)
   - Clear used words for new round (or persist, configurable)
   - Broadcast `RoundEnded` with round scores

2. **Game completion** (`backend/src/websocket/handler.rs`)
   - After round 5 → game ends
   - Calculate final scores
   - Determine winner (handle ties)
   - Store final results in database
   - Broadcast `GameEnded` with final standings

3. **Results screen** (`frontend/index.html`, `frontend/js/game-ui.js`)
   - Show final scores and rankings
   - Highlight winner
   - "Play Again" button → return to lobby
   - "Leave" button → disconnect

### Acceptance Criteria
- [ ] Game properly transitions between 5 rounds
- [ ] Final results display correctly
- [ ] Winner is clearly shown
- [ ] Players can start a new game

---

## Day 5: Dictionary & Validation Polish

### Tasks
1. **Dictionary population** (`backend/src/dictionary/mod.rs`)
   - Load word list on server startup
   - Use efficient data structure (HashSet or Trie)
   - Include common English words (50k-100k minimum)
   - Option to load from file or database

2. **Enhanced validation**
   - Minimum word length (3 letters)
   - Maximum word length (grid constraint)
   - Case-insensitive matching
   - Better error messages ("Word too short", "Not in dictionary", "Already used")

3. **Anti-cheat basics**
   - Server-authoritative: all validation server-side
   - Verify path matches grid positions
   - Rate limit word submissions

### Acceptance Criteria
- [ ] Real dictionary words work
- [ ] Made-up words are rejected
- [ ] Clear error feedback to players
- [ ] No client-side cheating possible

---

## Day 6: Reconnection & Edge Cases

### Tasks
1. **Player reconnection** (`backend/src/websocket/handler.rs`)
   - Detect reconnecting player (same user_id)
   - Send current game state on reconnect
   - Resume from correct turn position
   - Handle reconnect during their turn

2. **Player disconnect handling**
   - If player disconnects during game:
     - Use grace period (60 seconds)
     - Auto-skip their turn if disconnected
     - Allow game to continue
   - If all players disconnect → pause game state

3. **Edge cases**
   - Host leaves → transfer host to next player
   - Last player leaves → end game
   - Player leaves mid-turn → skip their turn
   - All players submit same word → first submit wins

### Acceptance Criteria
- [ ] Disconnected players can rejoin in progress game
- [ ] Game continues if some players leave
- [ ] No stuck game states

---

## Day 7: Testing & Polish

### Tasks
1. **Integration testing**
   - Test complete game flow end-to-end
   - Test multiplayer with 2, 4, 6 players
   - Test all edge cases from Day 6
   - Fix any bugs discovered

2. **UI/UX polish**
   - Add simple animations for word scoring
   - Sound effects placeholder (optional)
   - Loading states for all async operations
   - Error toasts for user feedback

3. **Performance check**
   - WebSocket message efficiency
   - Database query optimization
   - Memory usage with multiple concurrent games

### Acceptance Criteria
- [ ] Full game playable without bugs
- [ ] Smooth user experience
- [ ] Ready for friends & family testing

---

# Week 2: Single Player & Leaderboards

**Goal**: Adventure mode AI opponents, persistent leaderboards, user stats

## Single Player / Adventure Mode

### AI Opponent System
- Implement AI word finding algorithm
- Three difficulty levels:
  - **Easy**: Finds 3-4 letter words, occasional misses
  - **Medium**: Finds 5-6 letter words consistently
  - **Hard**: Finds optimal words, rarely beatable
- AI thinking delay for natural feel (1-3 seconds)

### Adventure Mode Structure
- 50 levels with increasing difficulty
- Level progression stored in database
- Star rating system (1-3 stars based on score)
- Unlock system: Complete level N to unlock N+1
- Boss levels every 10 levels (special rules)

### Practice Mode
- Solo play against AI
- No level restrictions
- Select AI difficulty
- Good for learning game mechanics

## Leaderboards

### Global Leaderboards
- All-time high scores
- Weekly high scores (reset Mondays)
- Monthly high scores
- Filter by game mode (multiplayer, adventure)

### Guild Leaderboards
- Per-Discord-server rankings
- Encourage competition within communities
- Guild nickname display

### Personal Stats
- Total games played
- Win/loss ratio
- Highest scoring word ever
- Average score per game
- Favorite words (most used)
- Longest word found

## Week 2 Technical Tasks

1. **AI Module** (`backend/src/ai/`)
   - Word finding algorithm (DFS/BFS on grid)
   - Difficulty scaling
   - Move selection strategy

2. **Adventure Mode** (`backend/src/adventure/`)
   - Level definitions (difficulty curves)
   - Progress tracking
   - Reward calculations

3. **Leaderboard API** (`backend/src/routes/leaderboard.rs`)
   - GET `/api/leaderboard/global`
   - GET `/api/leaderboard/guild/{guild_id}`
   - GET `/api/users/{user_id}/stats`

4. **Frontend Screens**
   - Adventure mode level select
   - Leaderboard display
   - User profile/stats page

---

# Weeks 3-4: Polish, Social & Monetization

**Goal**: Production-ready game with social features and revenue model

## Week 3: Social Features & Polish

### Friends & Social
- Challenge friends directly (via Discord)
- Share game results to Discord channel
- Spectator mode for ongoing games
- Game replays (view past games move-by-move)

### UI/UX Improvements
- Full animation suite
- Sound effects and music
- Haptic feedback (mobile)
- Accessibility improvements (colorblind mode, screen reader)
- Tutorial for new players
- Onboarding flow

### Game Variants
- **Speed Mode**: 30-second turns
- **Blitz Mode**: All players submit simultaneously
- **Team Mode**: 2v2 with shared word pool
- **Custom Rules**: Configurable by lobby host

### Quality of Life
- Game invites via Discord DMs
- Quick rematch option
- Lobby chat improvements
- Emoji reactions to words

## Week 4: Monetization & Launch Prep

### Monetization Options

#### Cosmetics (Non-Pay-to-Win)
- **Grid Themes**: Different visual styles for game board
- **Letter Styles**: Custom fonts/colors for letters
- **Victory Animations**: Celebration effects when winning
- **Profile Badges**: Show off achievements
- **Sound Packs**: Alternative sound effects

#### Premium Features
- **Ad-Free Experience**: Remove any ads (if implemented)
- **Extended Stats**: Detailed analytics and history
- **Custom Lobbies**: More customization options
- **Early Access**: New features first

#### Battle Pass (Seasonal)
- Free and premium tracks
- Cosmetic rewards for playing
- Seasonal themes (holiday events)
- Encourages daily engagement

### Technical Implementation
- Payment integration (Discord's IAP or Stripe)
- Inventory system for owned items
- Cosmetic application system
- Premium status tracking

### Launch Preparation
- Load testing (100+ concurrent players)
- Security audit
- Discord App Directory submission
- Marketing assets (screenshots, video)
- Support system setup
- Analytics integration

### Post-Launch Monitoring
- Error tracking (Sentry or similar)
- Performance monitoring
- User feedback collection
- Rapid bug fix pipeline

---

# Success Metrics

## Week 1 (MVP)
- [ ] 5+ complete multiplayer games with no crashes
- [ ] Full game loop working (lobby → game → results → lobby)
- [ ] Real-time synchronization < 100ms latency

## Week 2 (Content)
- [ ] AI beats testers 50% of time on Medium difficulty
- [ ] 50 adventure levels playable
- [ ] Leaderboards showing real data

## Weeks 3-4 (Production)
- [ ] < 1% crash rate in testing
- [ ] 100+ concurrent players supported
- [ ] Monetization generating revenue
- [ ] Discord App Directory approval

---

# Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Dictionary coverage | Use established word lists (TWL, SOWPODS) |
| WebSocket scalability | Design for horizontal scaling from start |
| Cheating | Server-authoritative design, rate limiting |
| Player toxicity | Rely on Discord's moderation, game is low-text |
| Scope creep | Strict MVP focus in Week 1 |

---

# Tech Debt to Address

1. Complete TODO comments in `handler.rs`
2. Add comprehensive integration tests
3. Implement proper logging throughout
4. Database connection pooling optimization
5. WebSocket message compression
6. Cache frequently accessed data (dictionary, leaderboards)

---

*Last Updated: 2025-11-28*
*Version: 1.0*
