use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub total_games: i32,
    pub total_wins: i32,
    pub total_score: i64,
    pub highest_word_score: i32,
    pub highest_word: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub total_games: i32,
    pub total_wins: i32,
    pub total_score: i64,
    pub win_rate: f32,
    pub highest_word_score: i32,
    pub highest_word: Option<String>,
}

impl User {
    pub fn win_rate(&self) -> f32 {
        if self.total_games == 0 {
            0.0
        } else {
            (self.total_wins as f32 / self.total_games as f32) * 100.0
        }
    }

    pub fn to_stats(&self) -> UserStats {
        UserStats {
            user_id: self.user_id,
            username: self.username.clone(),
            avatar_url: self.avatar_url.clone(),
            total_games: self.total_games,
            total_wins: self.total_wins,
            total_score: self.total_score,
            win_rate: self.win_rate(),
            highest_word_score: self.highest_word_score,
            highest_word: self.highest_word.clone(),
        }
    }
}
