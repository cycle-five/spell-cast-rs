-- Add refresh_token column to users table
ALTER TABLE users ADD COLUMN refresh_token TEXT;
ALTER TABLE users ADD COLUMN token_expires_at TIMESTAMP WITH TIME ZONE;

-- Create index for token expiration queries
CREATE INDEX idx_users_token_expires ON users(token_expires_at) WHERE token_expires_at IS NOT NULL;
