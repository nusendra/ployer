use anyhow::Result;
use ployer_core::models::DeployKey;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct DeployKeyRepository {
    pool: SqlitePool,
}

impl DeployKeyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        application_id: &str,
        public_key: &str,
        private_key_encrypted: &str,
    ) -> Result<DeployKey> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO deploy_keys (id, application_id, public_key, private_key_encrypted, created_at)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(application_id)
        .bind(public_key)
        .bind(private_key_encrypted)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.find_by_application(application_id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created deploy key"))
    }

    pub async fn find_by_application(&self, application_id: &str) -> Result<Option<DeployKey>> {
        let row = sqlx::query_as::<_, DeployKeyRow>(
            "SELECT id, application_id, public_key, private_key_encrypted, created_at
             FROM deploy_keys WHERE application_id = ?"
        )
        .bind(application_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn delete(&self, application_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM deploy_keys WHERE application_id = ?")
            .bind(application_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct DeployKeyRow {
    id: String,
    application_id: String,
    public_key: String,
    private_key_encrypted: String,
    created_at: String,
}

impl From<DeployKeyRow> for DeployKey {
    fn from(row: DeployKeyRow) -> Self {
        DeployKey {
            id: row.id,
            application_id: row.application_id,
            public_key: row.public_key,
            private_key_encrypted: row.private_key_encrypted,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }
}
