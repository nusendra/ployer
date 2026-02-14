use anyhow::Result;
use chrono::Utc;
use ployer_core::models::{Deployment, DeploymentStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct DeploymentRepository {
    pool: SqlitePool,
}

impl DeploymentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new deployment
    pub async fn create(
        &self,
        application_id: &str,
        server_id: &str,
        commit_sha: Option<&str>,
        commit_message: Option<&str>,
        image_tag: &str,
    ) -> Result<Deployment> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let status = DeploymentStatus::Queued;
        let status_str = status.as_str();
        let now_str = now.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO deployments (
                id, application_id, server_id, commit_sha, commit_message,
                status, image_tag, started_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            application_id,
            server_id,
            commit_sha,
            commit_message,
            status_str,
            image_tag,
            now_str
        )
        .execute(&self.pool)
        .await?;

        Ok(Deployment {
            id,
            application_id: application_id.to_string(),
            server_id: server_id.to_string(),
            commit_sha: commit_sha.map(|s| s.to_string()),
            commit_message: commit_message.map(|s| s.to_string()),
            status,
            build_log: None,
            container_id: None,
            image_tag: image_tag.to_string(),
            started_at: now,
            finished_at: None,
        })
    }

    /// Find deployment by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Deployment>> {
        let row = sqlx::query!(
            r#"
            SELECT id, application_id, server_id, commit_sha, commit_message,
                   status, build_log, container_id, image_tag, started_at, finished_at
            FROM deployments
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Deployment {
            id: r.id,
            application_id: r.application_id,
            server_id: r.server_id,
            commit_sha: r.commit_sha,
            commit_message: r.commit_message,
            status: DeploymentStatus::from_str(&r.status),
            build_log: r.build_log,
            container_id: r.container_id,
            image_tag: r.image_tag,
            started_at: r.started_at.parse().unwrap(),
            finished_at: r.finished_at.and_then(|f| f.parse().ok()),
        }))
    }

    /// List all deployments (optionally filtered by application)
    pub async fn list(&self, application_id: Option<&str>) -> Result<Vec<Deployment>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, application_id, server_id, commit_sha, commit_message,
                   status, build_log, container_id, image_tag, started_at, finished_at
            FROM deployments
            WHERE (? IS NULL OR application_id = ?)
            ORDER BY started_at DESC
            "#,
            application_id,
            application_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Deployment {
                id: r.id,
                application_id: r.application_id,
                server_id: r.server_id,
                commit_sha: r.commit_sha,
                commit_message: r.commit_message,
                status: DeploymentStatus::from_str(&r.status),
                build_log: r.build_log,
                container_id: r.container_id,
                image_tag: r.image_tag,
                started_at: r.started_at.parse().unwrap(),
                finished_at: r.finished_at.and_then(|f| f.parse().ok()),
            })
            .collect())
    }

    /// Update deployment status
    pub async fn update_status(&self, id: &str, status: DeploymentStatus) -> Result<()> {
        let status_str = status.as_str();
        let finished_at = if matches!(
            status,
            DeploymentStatus::Running | DeploymentStatus::Failed | DeploymentStatus::Cancelled
        ) {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        if let Some(finished) = &finished_at {
            sqlx::query!(
                "UPDATE deployments SET status = ?, finished_at = ? WHERE id = ?",
                status_str,
                finished,
                id
            )
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE deployments SET status = ? WHERE id = ?",
                status_str,
                id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Append to build log
    pub async fn append_log(&self, id: &str, log_line: &str) -> Result<()> {
        let line_with_newline = format!("{}\n", log_line);
        sqlx::query!(
            r#"
            UPDATE deployments
            SET build_log = COALESCE(build_log || ?, ?)
            WHERE id = ?
            "#,
            line_with_newline,
            line_with_newline,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Set container ID for deployment
    pub async fn set_container_id(&self, id: &str, container_id: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE deployments SET container_id = ? WHERE id = ?",
            container_id,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the latest successful deployment for an application
    pub async fn get_latest_running(&self, application_id: &str) -> Result<Option<Deployment>> {
        let row = sqlx::query!(
            r#"
            SELECT id, application_id, server_id, commit_sha, commit_message,
                   status, build_log, container_id, image_tag, started_at, finished_at
            FROM deployments
            WHERE application_id = ? AND status = 'running'
            ORDER BY started_at DESC
            LIMIT 1
            "#,
            application_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Deployment {
            id: r.id,
            application_id: r.application_id,
            server_id: r.server_id,
            commit_sha: r.commit_sha,
            commit_message: r.commit_message,
            status: DeploymentStatus::from_str(&r.status),
            build_log: r.build_log,
            container_id: r.container_id,
            image_tag: r.image_tag,
            started_at: r.started_at.parse().unwrap(),
            finished_at: r.finished_at.and_then(|f| f.parse().ok()),
        }))
    }

    /// Cancel a deployment (if it's still in progress)
    pub async fn cancel(&self, id: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query!(
            r#"
            UPDATE deployments
            SET status = 'cancelled', finished_at = ?
            WHERE id = ? AND status NOT IN ('running', 'failed', 'cancelled', 'rolled_back')
            "#,
            now,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
