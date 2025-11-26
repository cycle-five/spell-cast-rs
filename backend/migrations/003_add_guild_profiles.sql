-- Add table for tracking per-guild nicknames
-- Discord allows users to have different nicknames in each server (guild)
CREATE TABLE IF NOT EXISTS user_guild_profiles (
    user_id BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    guild_id BIGINT NOT NULL,
    nickname VARCHAR(255),
    -- Cache when we last fetched this info from Discord
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (user_id, guild_id)
);

-- Index for looking up all users in a guild
CREATE INDEX idx_user_guild_profiles_guild ON user_guild_profiles(guild_id);

-- Function to update user_guild_profiles timestamp
CREATE OR REPLACE FUNCTION update_user_guild_profiles_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-update timestamp
CREATE TRIGGER update_user_guild_profiles_timestamp
    BEFORE UPDATE ON user_guild_profiles
    FOR EACH ROW
    EXECUTE FUNCTION update_user_guild_profiles_updated_at();

-- Add comment explaining the display name priority
COMMENT ON TABLE user_guild_profiles IS 'Stores per-guild nicknames. Display priority: guild nickname > global_name > username';
