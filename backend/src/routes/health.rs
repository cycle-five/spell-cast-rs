use axum::Json;
use serde_json::{json, Value};

/// Health check endpoint
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "spell-cast-backend",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
