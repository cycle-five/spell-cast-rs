# Copilot Review Comments from PR #8 - Issues to Create

These are the non-critical but valid suggestions from Copilot's review of PR #8 (OAuth2 implementation).
Create these as GitHub issues to address in follow-up work.

---

## Issue 1: Store and use Discord refresh tokens for token refresh functionality

**Labels:** `enhancement`, `oauth`, `copilot-suggestion`

**Description:**

The `refresh_token` field in `DiscordTokenResponse` is currently retrieved from Discord but never stored or used.

**Current Behavior:**
- OAuth2 code exchange receives a refresh token from Discord
- The refresh token is immediately discarded
- No token refresh functionality exists

**Suggested Implementation:**
1. Add a `refresh_token` field to the `users` table (encrypted)
2. Store the refresh token when creating/updating user records
3. Implement a token refresh endpoint that uses the stored refresh token
4. Handle token expiration gracefully by refreshing when needed
5. Consider implementing automatic token refresh before expiration

**Location:** `backend/src/routes/auth.rs:38`

**Priority:** Low - Enhancement for better user experience

**Benefits:**
- Users won't need to re-authenticate after access token expires
- Better user experience for long-running sessions
- Follows OAuth2 best practices

**Security Considerations:**
- Refresh tokens should be encrypted at rest
- Implement token rotation (issue new refresh token on each refresh)
- Add refresh token revocation mechanism

---

## Issue 2: Remove global dead_code lint allowance

**Labels:** `code-quality`, `copilot-suggestion`

**Description:**

Globally allowing `dead_code` lint can hide genuinely unused code that should be removed.

**Current Behavior:**
- `#![allow(dead_code)]` is set globally in `backend/Cargo.toml`
- This suppresses warnings for all unused code across the entire codebase

**Suggested Implementation:**
1. Remove the global `#![allow(dead_code)]` from `Cargo.toml`
2. Identify specific items that legitimately need the allowance
3. Use `#[allow(dead_code)]` on those specific items with comments explaining why
4. Remove any genuinely unused code

**Location:** `backend/Cargo.toml:83`

**Priority:** Low - Code quality improvement

**Benefits:**
- Helps identify and remove unused code
- Makes intentional allowances more explicit
- Reduces codebase bloat over time

---

## Issue 3: Reuse reqwest::Client across requests for connection pooling

**Labels:** `performance`, `copilot-suggestion`

**Description:**

Creating a new `reqwest::Client` on every request is inefficient as it doesn't reuse connection pools.

**Current Behavior:**
- A new `reqwest::Client` is created in both `exchange_code_with_discord()` and `get_discord_user_info()`
- Each client creates its own connection pool
- Connections are not reused between requests

**Suggested Implementation:**
1. Add a `http_client: reqwest::Client` field to `AppState`
2. Initialize it once in `main.rs` when creating the app state
3. Reuse the shared client in OAuth functions: `let client = &state.client;`
4. Remove the `reqwest::Client::new()` calls

**Locations:**
- `backend/src/routes/auth.rs:116`
- `backend/src/routes/auth.rs:145`

**Priority:** Medium - Performance optimization

**Benefits:**
- Better performance through connection reuse
- Lower latency for OAuth requests
- Reduced resource usage

**Example Implementation:**
```rust
// In main.rs
let http_client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(30))
    .build()?;

let state = Arc::new(AppState {
    config,
    db,
    dictionary,
    active_games: DashMap::new(),
    http_client,
});

// In auth.rs
async fn exchange_code_with_discord(
    state: &AppState,
    code: &str,
) -> anyhow::Result<DiscordTokenResponse> {
    let response = state.http_client
        .post("https://discord.com/api/oauth2/token")
        // ...
}
```

---

## Issue 4: Remove or document unused Discord user fields

**Labels:** `code-quality`, `copilot-suggestion`

**Description:**

The `discriminator` and `global_name` fields are added to `DiscordUser` but never used in the code.

**Current Behavior:**
- `discriminator` field is present but unused (Discord is phasing out discriminators)
- `global_name` field is present but unused
- Only `id`, `username`, and `avatar` are actually used

**Suggested Implementation:**

**Option A:** Remove unused fields
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub avatar: Option<String>,
}
```

**Option B:** Keep them and add documentation
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub avatar: Option<String>,
    /// Currently unused; kept for future display name feature
    pub global_name: Option<String>,
    /// Currently unused; Discord is phasing out discriminators
    pub discriminator: Option<String>,
}
```

**Location:** `backend/src/routes/auth.rs:23`

**Priority:** Low - Code cleanliness

**Recommendation:**
- Remove `discriminator` (being phased out by Discord)
- Keep `global_name` with documentation if you plan to use it for display purposes
- Add a comment explaining why fields are present if not immediately used

---

## CRITICAL Issue (Fix Before Merging): Discord user IDs should use u64 instead of i64

**Labels:** `bug`, `critical`, `oauth`

**Description:**

Discord user IDs are u64 values (snowflakes) that can exceed i64::MAX. Parsing to i64 could fail for large user IDs.

**Current Behavior:**
```rust
let user_id = discord_user
    .id
    .parse::<i64>()  // ❌ Can overflow
```

**Problem:**
- Discord snowflakes are 64-bit unsigned integers
- Some valid Discord user IDs will be > i64::MAX (9,223,372,036,854,775,807)
- Parsing will fail with `ParseIntError` for these users
- This could affect real users with high-numbered accounts

**Required Fix:**

**Option 1:** Use i64 with wrapping (quick fix)
```rust
let user_id = discord_user.id.parse::<u64>()? as i64;
```

**Option 2:** Update database schema to use BIGINT UNSIGNED or DECIMAL (better)
```sql
ALTER TABLE users ALTER COLUMN user_id TYPE NUMERIC(20,0);
```
```rust
use sqlx::types::BigDecimal;
let user_id: BigDecimal = discord_user.id.parse()?;
```

**Option 3:** Store as String in database (simplest)
```rust
// Keep user_id as String throughout
```

**Location:** `backend/src/routes/auth.rs:71`

**Priority:** CRITICAL - Must fix before production

**Recommendation:**
For immediate fix: Use Option 1 (accept potential negative numbers)
For long-term: Use Option 3 (store as String or NUMERIC)

---

## Commands to Create Issues

You can use these `gh` commands to create the issues:

```bash
# Issue 1: Refresh tokens
gh issue create \
  --title "Store and use Discord refresh tokens for token refresh functionality" \
  --label "enhancement,oauth,copilot-suggestion" \
  --body-file <(cat <<'EOF'
The `refresh_token` field in `DiscordTokenResponse` is currently retrieved from Discord but never stored or used.

## Current Behavior
- OAuth2 code exchange receives a refresh token from Discord
- The refresh token is immediately discarded
- No token refresh functionality exists

## Suggested Implementation
1. Add a `refresh_token` field to the `users` table (encrypted)
2. Store the refresh token when creating/updating user records
3. Implement a token refresh endpoint
4. Handle token expiration gracefully

Location: `backend/src/routes/auth.rs:38`
Priority: Low - Enhancement

See COPILOT_REVIEW_ISSUES.md for full details.
EOF
)

# Issue 2: Dead code lint
gh issue create \
  --title "Remove global dead_code lint allowance" \
  --label "code-quality,copilot-suggestion" \
  --body-file <(cat <<'EOF'
Globally allowing `dead_code` lint can hide genuinely unused code that should be removed.

## Suggested Implementation
1. Remove global `#![allow(dead_code)]` from `Cargo.toml`
2. Use `#[allow(dead_code)]` on specific items with comments
3. Remove genuinely unused code

Location: `backend/Cargo.toml:83`
Priority: Low - Code quality

See COPILOT_REVIEW_ISSUES.md for full details.
EOF
)

# Issue 3: reqwest::Client reuse
gh issue create \
  --title "Reuse reqwest::Client across requests for connection pooling" \
  --label "performance,copilot-suggestion" \
  --body-file <(cat <<'EOF'
Creating a new `reqwest::Client` on every request is inefficient.

## Suggested Implementation
1. Add `http_client: reqwest::Client` to `AppState`
2. Initialize once in `main.rs`
3. Reuse across OAuth functions

Locations: `backend/src/routes/auth.rs:116`, `backend/src/routes/auth.rs:145`
Priority: Medium - Performance

See COPILOT_REVIEW_ISSUES.md for full details.
EOF
)

# Issue 4: Unused fields
gh issue create \
  --title "Remove or document unused Discord user fields" \
  --label "code-quality,copilot-suggestion" \
  --body-file <(cat <<'EOF'
The `discriminator` and `global_name` fields in `DiscordUser` are unused.

## Options
- Remove unused fields for cleaner code
- Keep with documentation if planned for future use

Location: `backend/src/routes/auth.rs:23`
Priority: Low - Code cleanliness

See COPILOT_REVIEW_ISSUES.md for full details.
EOF
)
```

Or create them via the GitHub web UI using the details above.
