use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User's display information in a specific guild
/// Discord allows users to have different nicknames per server
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserGuildProfile {
    pub user_id: i64,
    pub guild_id: i64,
    /// Guild-specific nickname (overrides global_name when present)
    pub nickname: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl UserGuildProfile {
    /// Get the display name for this user in this guild
    /// Priority: guild nickname > global_name > username
    pub fn display_name<'a>(&'a self, user: &'a super::User) -> &'a str {
        self.nickname
            .as_deref()
            .or(user.global_name.as_deref())
            .unwrap_or(&user.username)
    }
}
