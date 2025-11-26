-- Add refresh_token and display name columns to users table
ALTER TABLE users ADD COLUMN refresh_token TEXT;
ALTER TABLE users ADD COLUMN token_expires_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE users ADD COLUMN global_name VARCHAR(255);

-- Create index for token expiration queries
CREATE INDEX idx_users_token_expires ON users(token_expires_at) WHERE token_expires_at IS NOT NULL;
