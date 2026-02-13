use anyhow::Result;
use ployer_core::models::ApiKey;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ApiKeyRepository {
    pool: SqlitePool,
}

impl ApiKeyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user_id: &str, name: &str, key_hash: &str) -> Result<ApiKey> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO api_keys (id, user_id, name, key_hash, created_at)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(key_hash)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.find_by_id(&id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created API key"))
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<ApiKey>> {
        let row = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, user_id, name, key_hash, last_used_at, created_at FROM api_keys WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>> {
        let row = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, user_id, name, key_hash, last_used_at, created_at FROM api_keys WHERE key_hash = ?"
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, user_id, name, key_hash, last_used_at, created_at FROM api_keys WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update_last_used(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: String,
    user_id: String,
    name: String,
    key_hash: String,
    last_used_at: Option<String>,
    created_at: String,
}

impl From<ApiKeyRow> for ApiKey {
    fn from(row: ApiKeyRow) -> Self {
        ApiKey {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            key_hash: row.key_hash,
            last_used_at: row.last_used_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }
}
