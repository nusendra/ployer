use anyhow::Result;
use ployer_core::models::EnvironmentVariable;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct EnvVarRepository {
    pool: SqlitePool,
}

impl EnvVarRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        application_id: &str,
        key: &str,
        value_encrypted: &str,
    ) -> Result<EnvironmentVariable> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO environment_variables (id, application_id, key, value_encrypted, created_at)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(application_id)
        .bind(key)
        .bind(value_encrypted)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.find_by_id(&id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created environment variable"))
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<EnvironmentVariable>> {
        let row = sqlx::query_as::<_, EnvVarRow>(
            "SELECT id, application_id, key, value_encrypted, created_at
             FROM environment_variables WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn find_by_application_and_key(
        &self,
        application_id: &str,
        key: &str,
    ) -> Result<Option<EnvironmentVariable>> {
        let row = sqlx::query_as::<_, EnvVarRow>(
            "SELECT id, application_id, key, value_encrypted, created_at
             FROM environment_variables WHERE application_id = ? AND key = ?"
        )
        .bind(application_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_by_application(&self, application_id: &str) -> Result<Vec<EnvironmentVariable>> {
        let rows = sqlx::query_as::<_, EnvVarRow>(
            "SELECT id, application_id, key, value_encrypted, created_at
             FROM environment_variables WHERE application_id = ? ORDER BY key ASC"
        )
        .bind(application_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(
        &self,
        application_id: &str,
        key: &str,
        value_encrypted: &str,
    ) -> Result<EnvironmentVariable> {
        sqlx::query(
            "UPDATE environment_variables
             SET value_encrypted = ?
             WHERE application_id = ? AND key = ?"
        )
        .bind(value_encrypted)
        .bind(application_id)
        .bind(key)
        .execute(&self.pool)
        .await?;

        self.find_by_application_and_key(application_id, key).await?
            .ok_or_else(|| anyhow::anyhow!("Environment variable not found"))
    }

    pub async fn delete(&self, application_id: &str, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM environment_variables WHERE application_id = ? AND key = ?")
            .bind(application_id)
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_all_for_application(&self, application_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM environment_variables WHERE application_id = ?")
            .bind(application_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct EnvVarRow {
    id: String,
    application_id: String,
    key: String,
    value_encrypted: String,
    created_at: String,
}

impl From<EnvVarRow> for EnvironmentVariable {
    fn from(row: EnvVarRow) -> Self {
        EnvironmentVariable {
            id: row.id,
            application_id: row.application_id,
            key: row.key,
            value_encrypted: row.value_encrypted,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }
}
