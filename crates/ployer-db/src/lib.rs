pub mod repositories;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tracing::info;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    info!("Database connected: {}", database_url);
    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let migrations = [
        include_str!("../../../migrations/001_initial.sql"),
        include_str!("../../../migrations/002_webhooks.sql"),
        include_str!("../../../migrations/003_health_check_results.sql"),
        include_str!("../../../migrations/004_settings.sql"),
        include_str!("../../../migrations/005_resource_limits.sql"),
    ];

    for migration_sql in &migrations {
        for statement in migration_sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt).execute(pool).await?;
            }
        }
    }

    info!("Migrations applied successfully");
    Ok(())
}

#[cfg(test)]
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");
    run_migrations(&pool).await.expect("Migrations failed");
    pool
}

#[cfg(test)]
mod tests;
