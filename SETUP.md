# Spell Cast - Setup Guide

This guide will help you set up the Spell Cast Discord Activity for local development.

## Prerequisites

1. **Rust** (1.75 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **PostgreSQL** (14 or later)
   ```bash
   # macOS
   brew install postgresql@14
   brew services start postgresql@14

   # Ubuntu/Debian
   sudo apt install postgresql postgresql-contrib
   sudo systemctl start postgresql

   # Windows
   # Download from https://www.postgresql.org/download/windows/
   ```

3. **Node.js** (18 or later)
   ```bash
   # Download from https://nodejs.org/
   # or use nvm:
   nvm install 18
   ```

4. **SQLx CLI** (for database migrations)
   ```bash
   cargo install sqlx-cli --no-default-features --features postgres
   ```

## Step 1: Database Setup

1. Create the database:
   ```bash
   createdb spellcast
   ```

2. If you need to set a password:
   ```bash
   psql postgres
   CREATE DATABASE spellcast;
   CREATE USER spellcast_user WITH PASSWORD 'your_password';
   GRANT ALL PRIVILEGES ON DATABASE spellcast TO spellcast_user;
   \q
   ```

## Step 2: Discord Application Setup

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)

2. Click "New Application" and name it "Spell Cast"

3. Go to "Activities" tab:
   - Click "Enable Activity"
   - Add development URL: `http://localhost:3000`

4. Go to "OAuth2" tab:
   - Copy your **Client ID**
   - Copy your **Client Secret**
   - Add redirect URI: `http://localhost:3000/api/auth/callback`
   - Select scopes: `identify`, `guilds`

5. Go to "Installation" tab:
   - Set install link to "Discord Provided Link"
   - Select installation contexts: "Guild Install", "User Install"

## Step 3: Backend Configuration

1. Navigate to backend directory:
   ```bash
   cd backend
   ```

2. Copy environment template:
   ```bash
   cp .env.example .env
   ```

3. Edit `.env` with your values:
   ```bash
   DATABASE_URL=postgresql://postgres:password@localhost:5432/spellcast
   DISCORD_CLIENT_ID=your_client_id_from_step_2
   DISCORD_CLIENT_SECRET=your_client_secret_from_step_2
   JWT_SECRET=generate_a_random_secret_here
   ```

4. Run database migrations:
   ```bash
   sqlx migrate run
   ```

5. Download a word dictionary:
   ```bash
   # Option 1: Simple word list
   wget https://raw.githubusercontent.com/dwyl/english-words/master/words_alpha.txt -O dictionary.txt

   # Option 2: Scrabble dictionary (better for games)
   wget https://raw.githubusercontent.com/redbo/scrabble/master/dictionary.txt -O dictionary.txt
   ```

## Step 4: Frontend Configuration

1. Navigate to frontend directory:
   ```bash
   cd ../frontend
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Copy environment template:
   ```bash
   cp .env.example .env
   ```

4. Edit `.env` with your Discord Client ID:
   ```bash
   VITE_DISCORD_CLIENT_ID=your_client_id_from_step_2
   ```

## Step 5: Running the Application

### Development Mode

You'll need two terminal windows:

**Terminal 1 - Backend:**
```bash
cd backend
cargo run
```

The backend will start on http://localhost:3000

**Terminal 2 - Frontend (optional if using backend to serve):**
```bash
cd frontend
npm run dev
```

For development, the backend serves the frontend files, so you only need to run the backend.

### Testing in Discord

1. Open Discord and join a voice channel

2. Click the "Rocket" icon (Activities) in the voice chat panel

3. If developing locally, you'll need to use Discord's development mode:
   - Enable Developer Mode in Discord settings
   - In your application settings, add your local URL
   - Use the "URL Mapping" feature for local testing

4. Alternatively, deploy to a public URL (see Deployment section)

## Step 6: Verify Everything Works

1. Check backend health:
   ```bash
   curl http://localhost:3000/health
   ```

   Should return:
   ```json
   {"status":"ok","service":"spell-cast-backend","version":"0.1.0"}
   ```

2. Check WebSocket connection:
   ```bash
   # Install wscat if you don't have it
   npm install -g wscat

   # Connect to WebSocket
   wscat -c ws://localhost:3000/ws
   ```

## Troubleshooting

### Database Connection Issues

```bash
# Check if PostgreSQL is running
pg_isready

# Reset database
dropdb spellcast
createdb spellcast
cd backend
sqlx migrate run
```

### Port Already in Use

```bash
# Find process using port 3000
lsof -i :3000

# Kill the process
kill -9 <PID>
```

### Dictionary Not Loading

Make sure `dictionary.txt` exists in the `backend/` directory:

```bash
cd backend
ls -lh dictionary.txt
```

### Discord SDK Errors

For local development, the Discord SDK may not work outside of Discord's iframe. The code includes a development mode fallback. Check browser console for errors.

## Next Steps

- [ ] Implement OAuth2 flow in `backend/src/routes/auth.rs`
- [ ] Implement game creation logic in WebSocket handler
- [ ] Add word validation and scoring
- [ ] Implement turn-based gameplay
- [ ] Add sound effects and animations
- [ ] Deploy to production

## Deployment

See [DEPLOYMENT.md](./DEPLOYMENT.md) for production deployment instructions.

## Development Tips

1. **Hot Reload**: Use `cargo watch` for auto-reloading Rust code:
   ```bash
   cargo install cargo-watch
   cargo watch -x run
   ```

2. **Database Inspection**:
   ```bash
   psql spellcast
   \dt  # List tables
   SELECT * FROM users;
   ```

3. **Logging**: Set `RUST_LOG` for detailed logs:
   ```bash
   RUST_LOG=debug cargo run
   ```

4. **Testing**: Run tests with:
   ```bash
   cargo test
   ```
