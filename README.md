# Spell Cast Redux - Discord Activity

A Rust-powered Discord Activity that recreates the word game Spell Cast Redux. Players connect letters on a 5x5 grid to form words and compete for the highest score!

## Features

- **Multiplayer Mode**: 2-6 players, 5 rounds, highest score wins
- **2v2 Team Mode**: Team-based competitive gameplay
- **Adventure Mode**: 50 single-player levels with AI opponents
- **Real-time Gameplay**: WebSocket-powered synchronization
- **Discord Integration**: Play directly in voice channels

## Tech Stack

### Backend
- **Rust** - High-performance game engine
- **Axum** - Modern web framework
- **PostgreSQL** - Database with SQLx
- **WebSocket** - Real-time communication
- **OAuth2** - Discord authentication

### Frontend
- **HTML/CSS/JavaScript** - Web-based UI
- **Discord Embedded App SDK** - Discord integration
- **WebSocket Client** - Real-time updates

## Project Structure

```
spell-cast-rs/
├── backend/           # Rust backend server
│   ├── src/
│   │   ├── routes/    # HTTP endpoints
│   │   ├── websocket/ # WebSocket handlers
│   │   ├── game/      # Game engine
│   │   ├── models/    # Database models
│   │   ├── db/        # Database layer
│   │   ├── dictionary/# Word validation
│   │   └── utils/     # Utilities
│   └── migrations/    # Database migrations
└── frontend/          # Web frontend
    ├── js/            # JavaScript modules
    ├── styles/        # CSS stylesheets
    └── assets/        # Images, sounds, fonts
```

## Setup

### Prerequisites

- Rust 1.75+ (`rustup`)
- PostgreSQL 14+
- Node.js 18+ (for frontend dependencies)
- Discord Application (from Discord Developer Portal)

### Backend Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/cycle-five/spell-cast-rs
   cd spell-cast-rs/backend
   ```

2. **Install SQLx CLI**
   ```bash
   cargo install sqlx-cli --no-default-features --features postgres
   ```

3. **Set up environment variables**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Create database**
   ```bash
   export DATABASE_URL=postgresql://postgres:password@localhost:5432/spellcast
   sqlx database setup
   ```

5. **Download word dictionary**
   ```bash
   # Download SOWPODS or TWL word list
   wget https://raw.githubusercontent.com/dwyl/english-words/master/words_alpha.txt -O dictionary.txt
   ```

6. **Run the server**
   ```bash
   cargo run
   ```

### Frontend Setup

1. **Navigate to frontend**
   ```bash
   cd ../frontend
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Start development server**
   ```bash
   npm run dev
   ```

### Discord Application Setup

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application
3. Enable "Embedded App SDK" in Activities tab
4. Add your app URL (e.g., `http://localhost:3000` for the Vite dev server)
5. Configure OAuth2:
   - Add redirect URL: `http://localhost:3001/api/auth/callback` (for development with Vite)
   - For production, add: `http://localhost:3000/api/auth/callback`
   - Copy Client ID and Client Secret to `.env`
   - Required scopes: `identify`, `guilds`

## Development

### Database Migrations

Create a new migration:
```bash
sqlx migrate add <migration_name>
```

Run migrations:
```bash
sqlx migrate run
```

### Testing

Run backend tests:
```bash
cd backend
cargo test
```

### Building for Production

Backend:
```bash
cd backend
cargo build --release
```

Frontend:
```bash
cd frontend
npm run build
```

## API Documentation

### WebSocket Protocol

**Client → Server**
- `create_game` - Create new game
- `join_game` - Join existing game
- `submit_word` - Submit a word
- `pass_turn` - Skip turn

**Server → Client**
- `game_state` - Full game state
- `player_joined` - Player joined
- `turn_update` - Turn changed
- `word_scored` - Valid word scored
- `game_over` - Game finished

See [API.md](./API.md) for full documentation.

## License

GPLv3

## Contributing

Contributions are welcome! Please open an issue or pull request.
