pub mod auth;
pub mod health;

use crate::AppState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health::health_check))
        .nest("/api", api_routes())
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/exchange", axum::routing::post(auth::exchange_code))
        .route("/auth/me", get(auth::get_current_user))
}
