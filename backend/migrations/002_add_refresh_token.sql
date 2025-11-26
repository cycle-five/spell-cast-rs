-- Add encrypted refresh_token and display name columns to users table
-- NOTE: The refresh_token_encrypted column must store refresh tokens encrypted at rest.
ALTER TABLE users ADD COLUMN refresh_token_encrypted TEXT;
ALTER TABLE users ADD COLUMN token_expires_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE users ADD COLUMN global_name VARCHAR(255);

-- Create index for token expiration queries
CREATE INDEX idx_users_token_expires ON users(token_expires_at) WHERE token_expires_at IS NOT NULL;
