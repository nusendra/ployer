use anyhow::Result;
use chrono::Utc;
use ployer_core::models::ContainerStats;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ContainerStatsRepository {
    pool: SqlitePool,
}

impl ContainerStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Record container stats
    pub async fn record(
        &self,
        container_id: &str,
        application_id: Option<&str>,
        cpu_percent: f64,
        memory_mb: f64,
        memory_limit_mb: Option<f64>,
        network_rx_mb: Option<f64>,
        network_tx_mb: Option<f64>,
    ) -> Result<ContainerStats> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO container_stats (
                id, container_id, application_id, cpu_percent, memory_mb,
                memory_limit_mb, network_rx_mb, network_tx_mb, recorded_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            container_id,
            application_id,
            cpu_percent,
            memory_mb,
            memory_limit_mb,
            network_rx_mb,
            network_tx_mb,
            now_str
        )
        .execute(&self.pool)
        .await?;

        Ok(ContainerStats {
            id,
            container_id: container_id.to_string(),
            application_id: application_id.map(|s| s.to_string()),
            cpu_percent,
            memory_mb,
            memory_limit_mb,
            network_rx_mb,
            network_tx_mb,
            recorded_at: now,
        })
    }

    /// Get stats for a container within a time range
    pub async fn get_stats(
        &self,
        container_id: &str,
        hours_ago: i64,
    ) -> Result<Vec<ContainerStats>> {
        let time_filter = format!("-{} hours", hours_ago);
        let rows = sqlx::query!(
            r#"
            SELECT id, container_id, application_id, cpu_percent, memory_mb,
                   memory_limit_mb, network_rx_mb, network_tx_mb, recorded_at
            FROM container_stats
            WHERE container_id = ?
              AND recorded_at >= datetime('now', ?)
            ORDER BY recorded_at ASC
            "#,
            container_id,
            time_filter
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ContainerStats {
                id: r.id,
                container_id: r.container_id,
                application_id: r.application_id,
                cpu_percent: r.cpu_percent,
                memory_mb: r.memory_mb,
                memory_limit_mb: r.memory_limit_mb,
                network_rx_mb: r.network_rx_mb,
                network_tx_mb: r.network_tx_mb,
                recorded_at: r.recorded_at.parse().unwrap(),
            })
            .collect())
    }

    /// Get stats for an application within a time range
    pub async fn get_app_stats(
        &self,
        application_id: &str,
        hours_ago: i64,
    ) -> Result<Vec<ContainerStats>> {
        let time_filter = format!("-{} hours", hours_ago);
        let rows = sqlx::query!(
            r#"
            SELECT id, container_id, application_id, cpu_percent, memory_mb,
                   memory_limit_mb, network_rx_mb, network_tx_mb, recorded_at
            FROM container_stats
            WHERE application_id = ?
              AND recorded_at >= datetime('now', ?)
            ORDER BY recorded_at ASC
            "#,
            application_id,
            time_filter
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ContainerStats {
                id: r.id,
                container_id: r.container_id,
                application_id: r.application_id,
                cpu_percent: r.cpu_percent,
                memory_mb: r.memory_mb,
                memory_limit_mb: r.memory_limit_mb,
                network_rx_mb: r.network_rx_mb,
                network_tx_mb: r.network_tx_mb,
                recorded_at: r.recorded_at.parse().unwrap(),
            })
            .collect())
    }

    /// Clean up old stats (keep only last N hours)
    pub async fn cleanup_old_stats(&self, hours: i64) -> Result<u64> {
        let time_filter = format!("-{} hours", hours);
        let result = sqlx::query!(
            r#"
            DELETE FROM container_stats
            WHERE recorded_at < datetime('now', ?)
            "#,
            time_filter
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
