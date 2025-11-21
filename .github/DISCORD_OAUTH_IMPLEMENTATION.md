# Discord OAuth Implementation Task

## Overview
Implement proper Discord OAuth2 authentication flow to replace the current placeholder/test implementation in the authentication routes.

## Current State
- JWT-based authentication is implemented for WebSocket connections
- Auth routes have placeholder implementations using test user data
- Configuration is set up for Discord OAuth (client ID, secret, redirect URI)
- `oauth2` crate is already included in dependencies

## Required Implementation

### 1. OAuth2 Code Exchange (`exchange_code` endpoint)
**Location:** `backend/src/routes/auth.rs` (lines 36-58)

**Current behavior:**
- Returns a test JWT token with hardcoded user ID (12345) and username ("test_user")
- Logs warning about unimplemented OAuth

**Required implementation:**
1. Use the `oauth2` crate to exchange the authorization code for an access token
2. Make a request to Discord's `/oauth2/token` endpoint with:
   - `client_id` from config
   - `client_secret` from config  
   - `grant_type=authorization_code`
   - `code` from request payload
   - `redirect_uri` from config
3. Handle Discord API errors appropriately
4. Return the Discord access token or JWT token with actual user data

### 2. User Info Retrieval (`get_current_user` endpoint)
**Location:** `backend/src/routes/auth.rs` (lines 61-77)

**Current behavior:**
- Returns user info from the JWT token claims only
- Does not fetch fresh data from Discord

**Required implementation:**
1. Option A: Fetch user info from Discord API using the access token
   - Call Discord's `/users/@me` endpoint
   - Update/create user record in database
   - Return user info with avatar URL from Discord
2. Option B: Fetch user info from local database
   - Query user record by user_id from JWT claims
   - Return cached user information
3. Handle cases where user doesn't exist or token is invalid

### 3. User Database Operations
**Related:** User model and database schema

**Required:**
1. Create or update user records when OAuth succeeds
2. Store/update Discord user information:
   - Discord user ID (as primary identifier)
   - Username
   - Avatar URL/hash
   - Any other relevant Discord user fields
3. Map Discord user IDs to internal user IDs if needed

### 4. Token Storage and Refresh
**Optional enhancement:**

Consider implementing:
1. Refresh token storage and handling
2. Token refresh flow when access token expires
3. Secure token storage in database if persisting sessions

## Configuration Requirements

Ensure these environment variables are set:
- `DISCORD_CLIENT_ID` - Discord application client ID
- `DISCORD_CLIENT_SECRET` - Discord application client secret  
- `DISCORD_REDIRECT_URI` - OAuth redirect URI (must match Discord app settings)
- `JWT_SECRET` - Secret for signing JWT tokens

## Testing Checklist

- [ ] OAuth code exchange successfully retrieves access token from Discord
- [ ] User info is fetched from Discord API
- [ ] User records are created/updated in database
- [ ] JWT tokens contain correct user information
- [ ] WebSocket authentication works with real user tokens
- [ ] Error cases are handled (invalid code, network errors, etc.)
- [ ] Token expiration is handled appropriately

## Code Locations

### Files to modify:
- `backend/src/routes/auth.rs` - Main OAuth implementation
- `backend/src/db/` - User database operations (may need new queries)
- `backend/src/models/user.rs` - User model (may need updates)

### Files to reference:
- `backend/src/config.rs` - Discord config structure
- `backend/src/auth.rs` - JWT token generation
- `backend/Cargo.toml` - `oauth2` crate already included

### Files to remove after implementation:
- Test constants `TEST_USER_ID` and `TEST_USERNAME` in `backend/src/routes/auth.rs`

## Resources

- Discord OAuth2 Documentation: https://discord.com/developers/docs/topics/oauth2
- Discord API Reference: https://discord.com/developers/docs/resources/user
- oauth2 crate docs: https://docs.rs/oauth2/

## Acceptance Criteria

1. ✅ OAuth code exchange successfully communicates with Discord
2. ✅ Real Discord user data is fetched and stored
3. ✅ JWT tokens contain actual user information from Discord
4. ✅ Test constants and placeholder code are removed
5. ✅ Error handling is robust for all OAuth failure scenarios
6. ✅ All existing tests pass
7. ✅ New tests are added for OAuth flow

## Priority
**Medium-High** - The current implementation works for testing but is not production-ready. This should be completed before any production deployment.
