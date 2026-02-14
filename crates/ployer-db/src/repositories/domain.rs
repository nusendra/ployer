use anyhow::Result;
use chrono::Utc;
use ployer_core::models::Domain;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct DomainRepository {
    pool: SqlitePool,
}

impl DomainRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new domain
    pub async fn create(
        &self,
        application_id: &str,
        domain: &str,
        is_primary: bool,
    ) -> Result<Domain> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let is_primary_int = if is_primary { 1 } else { 0 };

        sqlx::query!(
            r#"
            INSERT INTO domains (id, application_id, domain, is_primary, ssl_active, created_at)
            VALUES (?, ?, ?, ?, 0, ?)
            "#,
            id,
            application_id,
            domain,
            is_primary_int,
            now_str
        )
        .execute(&self.pool)
        .await?;

        Ok(Domain {
            id,
            application_id: application_id.to_string(),
            domain: domain.to_string(),
            is_primary,
            ssl_active: false,
            created_at: now,
        })
    }

    /// Find domain by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Domain>> {
        let row = sqlx::query!(
            r#"
            SELECT id, application_id, domain, is_primary, ssl_active, created_at
            FROM domains
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Domain {
            id: r.id,
            application_id: r.application_id,
            domain: r.domain,
            is_primary: r.is_primary != 0,
            ssl_active: r.ssl_active != 0,
            created_at: r.created_at.parse().unwrap(),
        }))
    }

    /// Find domain by domain name
    pub async fn find_by_domain(&self, domain: &str) -> Result<Option<Domain>> {
        let row = sqlx::query!(
            r#"
            SELECT id, application_id, domain, is_primary, ssl_active, created_at
            FROM domains
            WHERE domain = ?
            "#,
            domain
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Domain {
            id: r.id,
            application_id: r.application_id,
            domain: r.domain,
            is_primary: r.is_primary != 0,
            ssl_active: r.ssl_active != 0,
            created_at: r.created_at.parse().unwrap(),
        }))
    }

    /// List all domains for an application
    pub async fn list_by_application(&self, application_id: &str) -> Result<Vec<Domain>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, application_id, domain, is_primary, ssl_active, created_at
            FROM domains
            WHERE application_id = ?
            ORDER BY is_primary DESC, created_at ASC
            "#,
            application_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Domain {
                id: r.id,
                application_id: r.application_id,
                domain: r.domain,
                is_primary: r.is_primary != 0,
                ssl_active: r.ssl_active != 0,
                created_at: r.created_at.parse().unwrap(),
            })
            .collect())
    }

    /// Update SSL status for a domain
    pub async fn update_ssl_status(&self, id: &str, ssl_active: bool) -> Result<()> {
        let ssl_active_int = if ssl_active { 1 } else { 0 };

        sqlx::query!(
            "UPDATE domains SET ssl_active = ? WHERE id = ?",
            ssl_active_int,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Set a domain as primary (and unset others for the same app)
    pub async fn set_primary(&self, id: &str) -> Result<()> {
        // First, get the application_id for this domain
        let domain = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Domain not found"))?;

        // Unset all primary flags for this application
        sqlx::query!(
            "UPDATE domains SET is_primary = 0 WHERE application_id = ?",
            domain.application_id
        )
        .execute(&self.pool)
        .await?;

        // Set this domain as primary
        sqlx::query!("UPDATE domains SET is_primary = 1 WHERE id = ?", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete a domain
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query!("DELETE FROM domains WHERE id = ?", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete domain by domain name
    pub async fn delete_by_domain(&self, domain: &str) -> Result<()> {
        sqlx::query!("DELETE FROM domains WHERE domain = ?", domain)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
