-- Add turn_order column to game_players for proper turn ordering
ALTER TABLE game_players ADD COLUMN IF NOT EXISTS turn_order INTEGER DEFAULT 0;

-- Create index for efficient ordering
CREATE INDEX IF NOT EXISTS idx_game_players_turn_order ON game_players(game_id, turn_order);
