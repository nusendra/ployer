use anyhow::Result;
use chrono::Utc;
use ployer_core::models::deployment::{HealthCheck, HealthCheckResult, HealthCheckStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct HealthCheckRepository {
    pool: SqlitePool,
}

impl HealthCheckRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create or update health check configuration for an application
    pub async fn upsert(
        &self,
        application_id: &str,
        path: &str,
        interval_seconds: i32,
        timeout_seconds: i32,
        healthy_threshold: i32,
        unhealthy_threshold: i32,
    ) -> Result<HealthCheck> {
        // Check if health check exists
        let existing = self.get(application_id).await?;

        if let Some(_existing) = existing {
            // Update existing
            sqlx::query!(
                r#"
                UPDATE health_checks
                SET path = ?, interval_seconds = ?, timeout_seconds = ?,
                    healthy_threshold = ?, unhealthy_threshold = ?
                WHERE application_id = ?
                "#,
                path,
                interval_seconds,
                timeout_seconds,
                healthy_threshold,
                unhealthy_threshold,
                application_id
            )
            .execute(&self.pool)
            .await?;

            self.get(application_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Failed to fetch updated health check"))
        } else {
            // Create new
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            sqlx::query!(
                r#"
                INSERT INTO health_checks (
                    id, application_id, path, interval_seconds, timeout_seconds,
                    healthy_threshold, unhealthy_threshold, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                id,
                application_id,
                path,
                interval_seconds,
                timeout_seconds,
                healthy_threshold,
                unhealthy_threshold,
                now
            )
            .execute(&self.pool)
            .await?;

            self.get(application_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Failed to fetch created health check"))
        }
    }

    /// Get health check configuration for an application
    pub async fn get(&self, application_id: &str) -> Result<Option<HealthCheck>> {
        let row = sqlx::query!(
            r#"
            SELECT id, application_id, path, interval_seconds, timeout_seconds,
                   healthy_threshold, unhealthy_threshold, created_at
            FROM health_checks
            WHERE application_id = ?
            "#,
            application_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| HealthCheck {
            id: r.id,
            application_id: r.application_id,
            path: r.path,
            interval_seconds: r.interval_seconds as i32,
            timeout_seconds: r.timeout_seconds as i32,
            healthy_threshold: r.healthy_threshold as i32,
            unhealthy_threshold: r.unhealthy_threshold as i32,
            created_at: r.created_at.parse().unwrap(),
        }))
    }

    /// List all health checks
    pub async fn list(&self) -> Result<Vec<HealthCheck>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, application_id, path, interval_seconds, timeout_seconds,
                   healthy_threshold, unhealthy_threshold, created_at
            FROM health_checks
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| HealthCheck {
                id: r.id,
                application_id: r.application_id,
                path: r.path,
                interval_seconds: r.interval_seconds as i32,
                timeout_seconds: r.timeout_seconds as i32,
                healthy_threshold: r.healthy_threshold as i32,
                unhealthy_threshold: r.unhealthy_threshold as i32,
                created_at: r.created_at.parse().unwrap(),
            })
            .collect())
    }

    /// Delete health check configuration
    pub async fn delete(&self, application_id: &str) -> Result<()> {
        sqlx::query!(
            "DELETE FROM health_checks WHERE application_id = ?",
            application_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record a health check result
    pub async fn record_result(
        &self,
        application_id: &str,
        container_id: &str,
        status: HealthCheckStatus,
        response_time_ms: Option<i32>,
        status_code: Option<i32>,
        error_message: Option<&str>,
    ) -> Result<HealthCheckResult> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let status_str = status.as_str();

        sqlx::query!(
            r#"
            INSERT INTO health_check_results (
                id, application_id, container_id, status,
                response_time_ms, status_code, error_message, checked_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            application_id,
            container_id,
            status_str,
            response_time_ms,
            status_code,
            error_message,
            now_str
        )
        .execute(&self.pool)
        .await?;

        Ok(HealthCheckResult {
            id,
            application_id: application_id.to_string(),
            container_id: container_id.to_string(),
            status,
            response_time_ms,
            status_code,
            error_message: error_message.map(|s| s.to_string()),
            checked_at: now,
        })
    }

    /// Get recent health check results for an application
    pub async fn get_recent_results(
        &self,
        application_id: &str,
        limit: i64,
    ) -> Result<Vec<HealthCheckResult>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, application_id, container_id, status,
                   response_time_ms, status_code, error_message, checked_at
            FROM health_check_results
            WHERE application_id = ?
            ORDER BY checked_at DESC
            LIMIT ?
            "#,
            application_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| HealthCheckResult {
                id: r.id,
                application_id: r.application_id,
                container_id: r.container_id,
                status: HealthCheckStatus::from_str(&r.status),
                response_time_ms: r.response_time_ms.map(|v| v as i32),
                status_code: r.status_code.map(|v| v as i32),
                error_message: r.error_message,
                checked_at: r.checked_at.parse().unwrap(),
            })
            .collect())
    }

    /// Get the latest health check status for an application
    pub async fn get_latest_status(&self, application_id: &str) -> Result<Option<HealthCheckStatus>> {
        let row = sqlx::query!(
            r#"
            SELECT status
            FROM health_check_results
            WHERE application_id = ?
            ORDER BY checked_at DESC
            LIMIT 1
            "#,
            application_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| HealthCheckStatus::from_str(&r.status)))
    }

    /// Clean up old health check results (keep only last N days)
    pub async fn cleanup_old_results(&self, days: i64) -> Result<u64> {
        let time_filter = format!("-{} days", days);
        let result = sqlx::query!(
            r#"
            DELETE FROM health_check_results
            WHERE checked_at < datetime('now', ?)
            "#,
            time_filter
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
