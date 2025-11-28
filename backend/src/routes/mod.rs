pub mod auth;
pub mod health;

use std::sync::Arc;

use axum::{routing::get, Router};

use crate::AppState;

pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health::health_check))
        .nest("/api", api_routes())
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/exchange", axum::routing::post(auth::exchange_code))
        .route("/auth/me", get(auth::get_current_user))
        .route("/auth/refresh", axum::routing::post(auth::refresh_token))
        .route("/auth/revoke", axum::routing::post(auth::revoke_token))
        .route("/auth/logout", axum::routing::post(auth::logout))
}
