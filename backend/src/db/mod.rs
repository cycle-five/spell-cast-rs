use sqlx::{postgres::PgPoolOptions, PgPool, Result};

pub mod queries;

pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
}
