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
    // Check if _migrations table already exists (i.e. tracking was set up before)
    let tracking_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_migrations'",
    )
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    )
    .execute(pool)
    .await?;

    let migrations: &[(&str, &str)] = &[
        ("001_initial", include_str!("../../../migrations/001_initial.sql")),
        ("002_webhooks", include_str!("../../../migrations/002_webhooks.sql")),
        ("003_health_check_results", include_str!("../../../migrations/003_health_check_results.sql")),
        ("004_settings", include_str!("../../../migrations/004_settings.sql")),
        ("005_resource_limits", include_str!("../../../migrations/005_resource_limits.sql")),
        ("006_template_apps", include_str!("../../../migrations/006_template_apps.sql")),
        ("007_wildcard_domains", include_str!("../../../migrations/007_wildcard_domains.sql")),
    ];

    // If tracking table was just created but the DB already has tables,
    // mark all existing migrations as applied to avoid re-running them.
    if !tracking_exists {
        let has_tables: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='users'",
        )
        .fetch_one(pool)
        .await?;

        if has_tables {
            for (name, _) in migrations {
                sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
                    .bind(name)
                    .execute(pool)
                    .await?;
            }
            info!("Migrations up to date (existing database, tracking initialized)");
            return Ok(());
        }
    }

    for (name, migration_sql) in migrations {
        let already_applied: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE name = ?",
        )
        .bind(name)
        .fetch_one(pool)
        .await?;

        if already_applied {
            continue;
        }

        for statement in migration_sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt).execute(pool).await?;
            }
        }

        sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
            .bind(name)
            .execute(pool)
            .await?;

        info!("Migration applied: {}", name);
    }

    info!("Migrations up to date");
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
