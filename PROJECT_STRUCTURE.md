# Project Structure

```
spell-cast-rs/
├── backend/                      # Rust backend server
│   ├── src/
│   │   ├── main.rs              # Entry point, server setup
│   │   ├── config.rs            # Configuration management
│   │   ├── routes/              # HTTP endpoints
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs          # OAuth2 authentication
│   │   │   └── health.rs        # Health check
│   │   ├── websocket/           # WebSocket handlers
│   │   │   ├── mod.rs
│   │   │   ├── handler.rs       # Connection handling
│   │   │   └── messages.rs      # Message types
│   │   ├── game/                # Game engine
│   │   │   ├── mod.rs
│   │   │   ├── grid.rs          # Grid generation
│   │   │   ├── validator.rs    # Word validation
│   │   │   └── scorer.rs        # Scoring logic
│   │   ├── models/              # Database models
│   │   │   ├── mod.rs
│   │   │   ├── user.rs          # User model
│   │   │   └── game.rs          # Game models
│   │   ├── db/                  # Database layer
│   │   │   ├── mod.rs
│   │   │   └── queries.rs       # SQL queries
│   │   ├── dictionary/          # Word dictionary
│   │   │   └── mod.rs
│   │   └── utils/               # Utilities
│   │       ├── mod.rs
│   │       └── letters.rs       # Letter values & distribution
│   ├── migrations/              # Database migrations
│   │   └── 001_initial_schema.sql
│   ├── Cargo.toml               # Rust dependencies
│   ├── .env.example             # Environment template
│   └── dictionary.txt           # Word list (to be downloaded)
│
├── frontend/                     # Web frontend (Discord Activity)
│   ├── js/
│   │   ├── main.js              # App initialization
│   │   ├── discord-sdk.js       # Discord SDK integration
│   │   ├── websocket.js         # WebSocket client
│   │   └── game-ui.js           # Game UI rendering
│   ├── styles/
│   │   ├── main.css             # Main styles
│   │   └── grid.css             # Game board styles
│   ├── assets/
│   │   ├── images/              # Images (empty for now)
│   │   ├── sounds/              # Sound effects (empty for now)
│   │   └── fonts/               # Custom fonts (empty for now)
│   ├── index.html               # Main HTML
│   ├── package.json             # Node dependencies
│   ├── vite.config.js           # Vite configuration
│   └── .env.example             # Frontend environment template
│
├── README.md                     # Project overview
├── SETUP.md                      # Setup instructions
├── PROJECT_STRUCTURE.md          # This file
└── .gitignore                    # Git ignore rules
```

## File Count Summary

- **Backend**: 20 Rust source files + 1 migration + 2 config files
- **Frontend**: 4 JavaScript modules + 2 CSS files + 1 HTML + 2 config files
- **Documentation**: 3 markdown files
- **Total**: ~35 files created

## Key Components

### Backend (Rust)

| Component | Status | Description |
|-----------|--------|-------------|
| HTTP Server | ✅ Setup | Axum-based REST API |
| WebSocket | ✅ Setup | Real-time game communication |
| Database | ✅ Schema | PostgreSQL with SQLx |
| Game Engine | ✅ Basic | Grid generation, validation, scoring |
| OAuth2 | 🚧 Placeholder | Discord authentication (TODO) |
| Dictionary | 🚧 Placeholder | Word list loader (download needed) |

### Frontend (JavaScript)

| Component | Status | Description |
|-----------|--------|-------------|
| Discord SDK | ✅ Setup | Activity integration |
| WebSocket Client | ✅ Complete | Real-time server communication |
| Game UI | ✅ Complete | Board rendering, tile selection |
| Screens | ✅ Complete | Loading, lobby, game, results |
| Styling | ✅ Complete | Discord-themed dark UI |

## Implementation Status

### ✅ Completed
- Project structure and build system
- Database schema and migrations
- Basic HTTP endpoints (health check)
- WebSocket message protocol
- Game engine fundamentals (grid, validation, scoring)
- Frontend UI and interactions
- Discord SDK integration (client-side)

### 🚧 In Progress / TODO
- OAuth2 flow implementation
- Game state management (in-memory + database sync)
- Turn-based gameplay logic
- Word dictionary integration
- Bot AI for adventure mode
- Sound effects and animations
- Leaderboard system
- 2v2 team mode
- Adventure mode (50 levels)

## Next Implementation Steps

1. **Complete OAuth2 Flow** (`backend/src/routes/auth.rs`)
   - Implement code exchange
   - Store access tokens
   - Verify Discord user info

2. **Game State Management** (new module: `backend/src/game/manager.rs`)
   - In-memory game sessions
   - Player join/leave logic
   - Turn rotation
   - Round progression

3. **Word Submission** (`backend/src/websocket/handler.rs`)
   - Validate word in dictionary
   - Check path validity
   - Calculate score
   - Update database
   - Broadcast to players

4. **Download Dictionary**
   ```bash
   cd backend
   wget https://raw.githubusercontent.com/redbo/scrabble/master/dictionary.txt
   ```

5. **Testing**
   - Unit tests for game engine
   - Integration tests for WebSocket
   - End-to-end testing in Discord

## Technology Stack

### Backend
- **Language**: Rust 2021 edition
- **Framework**: Axum 0.7
- **Database**: PostgreSQL + SQLx
- **Runtime**: Tokio (async)
- **WebSocket**: tokio-tungstenite

### Frontend
- **Language**: JavaScript (ES6 modules)
- **Build Tool**: Vite 5
- **SDK**: @discord/embedded-app-sdk
- **Styling**: Vanilla CSS (Discord theme)

### Infrastructure
- **Development**: Local PostgreSQL + Cargo + npm
- **Production**: TBD (Railway, Fly.io, or custom VPS)

## Database Schema

See `backend/migrations/001_initial_schema.sql` for full schema.

**Main Tables**:
- `users` - Player profiles and statistics
- `games` - Game sessions
- `game_players` - Player participation
- `game_boards` - Current board state
- `game_moves` - Move history
- `adventure_progress` - Adventure mode progress
- `dictionary` - Word list
- `leaderboard` - Rankings

## API Endpoints

### HTTP (REST)
- `GET /health` - Health check
- `POST /api/auth/exchange` - Exchange OAuth code
- `GET /api/auth/me` - Get current user

### WebSocket (Real-time)
**Client → Server**:
- `create_game` - Create new game
- `join_game` - Join existing game
- `start_game` - Start game
- `submit_word` - Submit word
- `pass_turn` - Skip turn

**Server → Client**:
- `game_state` - Full game state
- `player_joined` - Player joined
- `turn_update` - Turn changed
- `word_scored` - Word accepted
- `invalid_word` - Word rejected
- `game_over` - Game finished

## Development Workflow

1. Start PostgreSQL: `brew services start postgresql`
2. Run backend: `cd backend && cargo run`
3. Frontend served by backend at `http://localhost:3000`
4. WebSocket at `ws://localhost:3000/ws`

## Build Commands

```bash
# Backend
cd backend
cargo build --release

# Frontend
cd frontend
npm run build

# Database
cd backend
sqlx migrate run
```
