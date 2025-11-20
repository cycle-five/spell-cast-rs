pub mod auth;
pub mod health;

use axum::{Router, routing::get};
use crate::AppState;
use std::sync::Arc;

pub fn create_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .nest("/api", api_routes(state.clone()))
}

fn api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/exchange", axum::routing::post(auth::exchange_code))
        .route("/auth/me", get(auth::get_current_user))
        .with_state(state)
}
