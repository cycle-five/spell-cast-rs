-- Initial database schema for Spell Cast

-- Users table
CREATE TABLE IF NOT EXISTS users (
    user_id BIGINT PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    avatar_url VARCHAR(512),
    total_games INTEGER DEFAULT 0,
    total_wins INTEGER DEFAULT 0,
    total_score BIGINT DEFAULT 0,
    highest_word_score INTEGER DEFAULT 0,
    highest_word VARCHAR(50),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Games table
CREATE TABLE IF NOT EXISTS games (
    game_id UUID PRIMARY KEY,
    guild_id BIGINT,
    channel_id BIGINT NOT NULL,
    game_mode VARCHAR(20) NOT NULL CHECK (game_mode IN ('multiplayer', '2v2', 'adventure')),
    state VARCHAR(20) NOT NULL CHECK (state IN ('waiting', 'active', 'finished', 'cancelled')),
    current_round INTEGER DEFAULT 1,
    max_rounds INTEGER DEFAULT 5,
    current_turn_player BIGINT,
    timer_enabled BOOLEAN DEFAULT FALSE,
    timer_duration INTEGER DEFAULT 30,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    finished_at TIMESTAMP WITH TIME ZONE
);

-- Game players
CREATE TABLE IF NOT EXISTS game_players (
    id SERIAL PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    team INTEGER,
    score INTEGER DEFAULT 0,
    is_bot BOOLEAN DEFAULT FALSE,
    bot_difficulty VARCHAR(20) CHECK (bot_difficulty IN ('easy', 'medium', 'hard')),
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(game_id, user_id)
);

-- Game boards (current state)
CREATE TABLE IF NOT EXISTS game_boards (
    game_id UUID PRIMARY KEY REFERENCES games(game_id) ON DELETE CASCADE,
    grid JSONB NOT NULL,
    used_words JSONB DEFAULT '[]'::JSONB,
    round_number INTEGER DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Game moves (history)
CREATE TABLE IF NOT EXISTS game_moves (
    id SERIAL PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    round_number INTEGER NOT NULL,
    word VARCHAR(50) NOT NULL,
    score INTEGER NOT NULL,
    positions JSONB NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Adventure progress
CREATE TABLE IF NOT EXISTS adventure_progress (
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    level INTEGER NOT NULL,
    completed BOOLEAN DEFAULT FALSE,
    stars INTEGER DEFAULT 0 CHECK (stars >= 0 AND stars <= 3),
    high_score INTEGER DEFAULT 0,
    attempts INTEGER DEFAULT 0,
    completed_at TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (user_id, level)
);

-- Leaderboard (materialized view data)
CREATE TABLE IF NOT EXISTS leaderboard (
    user_id BIGINT PRIMARY KEY REFERENCES users(user_id),
    rank INTEGER NOT NULL,
    total_score BIGINT NOT NULL,
    total_wins INTEGER NOT NULL,
    total_games INTEGER NOT NULL,
    win_rate DECIMAL(5,2),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Word dictionary
CREATE TABLE IF NOT EXISTS dictionary (
    word VARCHAR(50) PRIMARY KEY,
    length INTEGER NOT NULL,
    frequency INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_games_state ON games(state);
CREATE INDEX IF NOT EXISTS idx_games_guild ON games(guild_id);
CREATE INDEX IF NOT EXISTS idx_games_channel ON games(channel_id);
CREATE INDEX IF NOT EXISTS idx_game_players_game ON game_players(game_id);
CREATE INDEX IF NOT EXISTS idx_game_players_user ON game_players(user_id);
CREATE INDEX IF NOT EXISTS idx_game_moves_game ON game_moves(game_id);
CREATE INDEX IF NOT EXISTS idx_game_moves_user ON game_moves(user_id);
CREATE INDEX IF NOT EXISTS idx_adventure_user ON adventure_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_adventure_level ON adventure_progress(level);
CREATE INDEX IF NOT EXISTS idx_dictionary_length ON dictionary(length);
CREATE INDEX IF NOT EXISTS idx_leaderboard_rank ON leaderboard(rank);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for updated_at
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_game_boards_updated_at
    BEFORE UPDATE ON game_boards
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
