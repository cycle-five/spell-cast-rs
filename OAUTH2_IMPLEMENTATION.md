# OAuth2 Implementation

The OAuth2 flow has been fully implemented for Discord authentication.

## Flow Overview

```
Frontend                    Backend                      Discord API
   |                           |                              |
   |  1. Request auth code     |                              |
   |-------------------------->|                              |
   |                           |  2. Redirect to Discord      |
   |                           |----------------------------->|
   |                           |                              |
   |  3. User authorizes       |                              |
   |<----------------------------------------------------------|
   |                           |                              |
   |  4. POST /api/auth/exchange with code                    |
   |-------------------------->|                              |
   |                           |  5. Exchange code for token  |
   |                           |----------------------------->|
   |                           |<-----------------------------|
   |                           |  6. Get user info            |
   |                           |----------------------------->|
   |                           |<-----------------------------|
   |                           |  7. Store user in DB         |
   |                           |                              |
   |  8. Return JWT token      |                              |
   |<--------------------------|                              |
```

## Endpoints

### POST /api/auth/exchange

Exchange a Discord authorization code for an access token.

**Request:**
```json
{
  "code": "discord_authorization_code"
}
```

**Response:**
```json
{
  "access_token": "jwt_token"
}
```

**Process:**
1. Exchanges authorization code with Discord OAuth2 API
2. Retrieves user information from Discord `/users/@me` endpoint
3. Creates or updates user in the database
4. Generates a JWT token for session management
5. Returns JWT token to client

### GET /api/auth/me

Get current authenticated user information.

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Response:**
```json
{
  "user_id": 123456789,
  "username": "discord_user",
  "avatar_url": "https://cdn.discordapp.com/avatars/..."
}
```

## Implementation Details

### Authentication Flow (`/backend/src/routes/auth.rs`)

1. **Code Exchange** (`exchange_code`)
   - Validates incoming authorization code
   - Calls Discord OAuth2 token endpoint
   - Exchanges code for access token
   - Handles errors from Discord API

2. **User Info Retrieval** (`get_discord_user_info`)
   - Uses Discord access token to fetch user data
   - Calls `/users/@me` endpoint
   - Parses Discord user response

3. **Database Storage**
   - Creates new user if not exists
   - Updates existing user information (username, avatar)
   - Uses `create_or_update_user` from database queries

4. **JWT Token Generation**
   - Creates JWT with user_id and username claims
   - Sets 24-hour expiration
   - Signs with JWT_SECRET from config

### Authentication Middleware (`/backend/src/auth.rs`)

The `AuthenticatedUser` extractor provides automatic authentication for routes:

```rust
pub async fn protected_route(
    user: AuthenticatedUser,  // Automatically validated
    State(state): State<Arc<AppState>>,
) -> Result<Json<Response>, StatusCode> {
    // user.user_id and user.username are available
}
```

**Token Sources:**
- Authorization header: `Bearer <token>`
- Query parameter: `?token=<token>`

**Validation:**
- Verifies JWT signature
- Checks token expiration
- Extracts user_id and username from claims

## Environment Variables

Required environment variables for OAuth2:

```bash
# Discord OAuth2
DISCORD_CLIENT_ID=your_client_id
DISCORD_CLIENT_SECRET=your_client_secret
DISCORD_REDIRECT_URI=http://localhost:3001/api/auth/callback

# JWT Secret (generate with: python3 -c "import secrets; print(secrets.token_hex(32))")
JWT_SECRET=your_secret_key_here
```

## Testing

### Testing with curl

1. **Get authorization code from Discord:**
   - Visit: `https://discord.com/oauth2/authorize?client_id=YOUR_CLIENT_ID&redirect_uri=YOUR_REDIRECT_URI&response_type=code&scope=identify%20guilds`
   - Authorize the application
   - Copy the `code` parameter from the redirect URL

2. **Exchange code for token:**
   ```bash
   curl -X POST http://localhost:3001/api/auth/exchange \
     -H "Content-Type: application/json" \
     -d '{"code": "YOUR_AUTH_CODE"}'
   ```

3. **Test authenticated endpoint:**
   ```bash
   curl http://localhost:3001/api/auth/me \
     -H "Authorization: Bearer YOUR_JWT_TOKEN"
   ```

### Testing with the Frontend

The frontend (`/frontend/js/discord-sdk.js`) handles the OAuth flow automatically:

1. Initializes Discord SDK
2. Requests authorization
3. Receives authorization code
4. Calls `/api/auth/exchange` endpoint
5. Stores JWT token for future requests

## Security Considerations

✅ **Implemented:**
- HTTPS required for production (Discord requirement)
- JWT tokens expire after 24 hours
- Tokens signed with secret key
- Authorization header validation
- Discord API error handling

⚠️ **Future Enhancements:**
- Token refresh mechanism
- Token revocation/blacklist
- Rate limiting on auth endpoints
- CSRF protection for auth flow
- Refresh token storage and rotation

## Error Handling

The implementation handles various error scenarios:

- **Invalid authorization code:** Returns 401 Unauthorized
- **Discord API errors:** Logs error and returns appropriate status
- **Database errors:** Returns 500 Internal Server Error
- **Invalid JWT:** Returns 401 Unauthorized
- **Expired JWT:** Returns 401 Unauthorized
- **User not found:** Returns 404 Not Found

All errors are logged with tracing for debugging.

## Database Schema

Users are stored in the `users` table:

```sql
CREATE TABLE users (
    user_id BIGINT PRIMARY KEY,          -- Discord user ID
    username VARCHAR(255) NOT NULL,       -- Discord username
    avatar_url VARCHAR(512),              -- Discord avatar URL
    total_games INT DEFAULT 0,
    total_wins INT DEFAULT 0,
    total_score BIGINT DEFAULT 0,
    highest_word_score INT DEFAULT 0,
    highest_word VARCHAR(50),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

On each login:
- New users are inserted
- Existing users have their username and avatar updated
- Last login timestamp is automatically updated

## Next Steps

With OAuth2 fully implemented, you can now:

1. **Test the complete authentication flow** from frontend to backend
2. **Protect WebSocket connections** by requiring authentication
3. **Add user-specific game features** (game history, stats, leaderboards)
4. **Implement multiplayer** with authenticated users
5. **Add refresh token support** for longer sessions

## Files Modified

- `/backend/src/routes/auth.rs` - OAuth2 endpoints implementation
- `/backend/src/auth.rs` - JWT generation and validation (already existed)
- `/backend/src/db/queries.rs` - User CRUD operations (already existed)
- `/backend/Cargo.toml` - Dependencies (already had all needed crates)
