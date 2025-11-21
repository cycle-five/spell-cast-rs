use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,      // User ID
    pub username: String, // Username
    pub exp: usize,       // Expiration time
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub username: String,
}

/// Extractor for authenticated users from JWT tokens
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = StatusCode;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let app_state = Arc::<AppState>::from_ref(state);

        // Try to extract token from Authorization header first
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(String::from)
            // If no Authorization header, try query parameter
            .or_else(|| {
                parts
                    .uri
                    .query()
                    .and_then(|q| {
                        serde_urlencoded::from_str::<Vec<(String, String)>>(q).ok()
                    })
                    .and_then(|params| {
                        params
                            .iter()
                            .find(|(k, _)| k == "token")
                            .map(|(_, v)| v.clone())
                    })
            });

        async move {
            let token = token.ok_or(StatusCode::UNAUTHORIZED)?;

            // Validate the JWT token
            let token_data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(app_state.config.security.jwt_secret.as_ref()),
                &Validation::default(),
            )
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

            let user_id = token_data
                .claims
                .sub
                .parse::<i64>()
                .map_err(|_| StatusCode::UNAUTHORIZED)?;

            Ok(AuthenticatedUser {
                user_id,
                username: token_data.claims.username,
            })
        }
    }
}

/// Generate a JWT token for a user
pub fn generate_token(
    user_id: i64,
    username: &str,
    jwt_secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: expiration as usize,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(jwt_secret.as_ref()),
    )
}
