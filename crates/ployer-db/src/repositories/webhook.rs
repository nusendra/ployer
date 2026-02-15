use anyhow::Result;
use chrono::Utc;
use ployer_core::models::{Webhook, WebhookProvider, WebhookDelivery, WebhookDeliveryStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct WebhookRepository {
    pool: SqlitePool,
}

impl WebhookRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new webhook
    pub async fn create(
        &self,
        application_id: &str,
        provider: WebhookProvider,
        secret: &str,
    ) -> Result<Webhook> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let provider_str = provider.as_str();

        sqlx::query!(
            r#"
            INSERT INTO webhooks (id, application_id, provider, secret, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, 1, ?, ?)
            "#,
            id,
            application_id,
            provider_str,
            secret,
            now_str,
            now_str
        )
        .execute(&self.pool)
        .await?;

        Ok(Webhook {
            id,
            application_id: application_id.to_string(),
            provider,
            secret: secret.to_string(),
            enabled: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Find webhook by application ID
    pub async fn find_by_application(&self, application_id: &str) -> Result<Option<Webhook>> {
        let row = sqlx::query!(
            r#"
            SELECT id, application_id, provider, secret, enabled, created_at, updated_at
            FROM webhooks
            WHERE application_id = ?
            "#,
            application_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Webhook {
            id: r.id,
            application_id: r.application_id,
            provider: WebhookProvider::from_str(&r.provider),
            secret: r.secret,
            enabled: r.enabled != 0,
            created_at: r.created_at.parse().unwrap(),
            updated_at: r.updated_at.parse().unwrap(),
        }))
    }

    /// Update webhook secret
    pub async fn update_secret(&self, application_id: &str, secret: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query!(
            "UPDATE webhooks SET secret = ?, updated_at = ? WHERE application_id = ?",
            secret,
            now,
            application_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Toggle webhook enabled status
    pub async fn toggle_enabled(&self, application_id: &str, enabled: bool) -> Result<()> {
        let enabled_int = if enabled { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();

        sqlx::query!(
            "UPDATE webhooks SET enabled = ?, updated_at = ? WHERE application_id = ?",
            enabled_int,
            now,
            application_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete webhook
    pub async fn delete(&self, application_id: &str) -> Result<()> {
        sqlx::query!("DELETE FROM webhooks WHERE application_id = ?", application_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a webhook delivery record
    pub async fn create_delivery(
        &self,
        webhook_id: &str,
        application_id: &str,
        provider: WebhookProvider,
        event_type: &str,
        branch: Option<&str>,
        commit_sha: Option<&str>,
        commit_message: Option<&str>,
        author: Option<&str>,
        status: WebhookDeliveryStatus,
        response_code: Option<i32>,
        error_message: Option<&str>,
        deployment_id: Option<&str>,
    ) -> Result<WebhookDelivery> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let provider_str = provider.as_str();
        let status_str = status.as_str();

        sqlx::query!(
            r#"
            INSERT INTO webhook_deliveries (
                id, webhook_id, application_id, provider, event_type,
                branch, commit_sha, commit_message, author,
                status, response_code, error_message, deployment_id, delivered_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            webhook_id,
            application_id,
            provider_str,
            event_type,
            branch,
            commit_sha,
            commit_message,
            author,
            status_str,
            response_code,
            error_message,
            deployment_id,
            now_str
        )
        .execute(&self.pool)
        .await?;

        Ok(WebhookDelivery {
            id,
            webhook_id: webhook_id.to_string(),
            application_id: application_id.to_string(),
            provider,
            event_type: event_type.to_string(),
            branch: branch.map(|s| s.to_string()),
            commit_sha: commit_sha.map(|s| s.to_string()),
            commit_message: commit_message.map(|s| s.to_string()),
            author: author.map(|s| s.to_string()),
            status,
            response_code,
            error_message: error_message.map(|s| s.to_string()),
            deployment_id: deployment_id.map(|s| s.to_string()),
            delivered_at: now,
        })
    }

    /// List webhook deliveries for an application
    pub async fn list_deliveries(&self, application_id: &str, limit: i64) -> Result<Vec<WebhookDelivery>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, webhook_id, application_id, provider, event_type,
                   branch, commit_sha, commit_message, author,
                   status, response_code, error_message, deployment_id, delivered_at
            FROM webhook_deliveries
            WHERE application_id = ?
            ORDER BY delivered_at DESC
            LIMIT ?
            "#,
            application_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| WebhookDelivery {
                id: r.id,
                webhook_id: r.webhook_id,
                application_id: r.application_id,
                provider: WebhookProvider::from_str(&r.provider),
                event_type: r.event_type,
                branch: r.branch,
                commit_sha: r.commit_sha,
                commit_message: r.commit_message,
                author: r.author,
                status: WebhookDeliveryStatus::from_str(&r.status),
                response_code: r.response_code.map(|c| c as i32),
                error_message: r.error_message,
                deployment_id: r.deployment_id,
                delivered_at: r.delivered_at.parse().unwrap(),
            })
            .collect())
    }
}
