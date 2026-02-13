use anyhow::Result;
use ployer_core::models::{Server, ServerStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ServerRepository {
    pool: SqlitePool,
}

impl ServerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        host: &str,
        port: u16,
        username: &str,
        ssh_key_encrypted: Option<&str>,
        is_local: bool,
    ) -> Result<Server> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let status = ServerStatus::Unknown.as_str();
        let is_local_int = if is_local { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO servers (id, name, host, port, username, ssh_key_encrypted, is_local, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(name)
        .bind(host)
        .bind(port as i64)
        .bind(username)
        .bind(ssh_key_encrypted)
        .bind(is_local_int)
        .bind(status)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.find_by_id(&id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created server"))
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Server>> {
        let row = sqlx::query_as::<_, ServerRow>(
            "SELECT id, name, host, port, username, ssh_key_encrypted, is_local, status, last_seen_at, created_at, updated_at
             FROM servers WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list(&self) -> Result<Vec<Server>> {
        let rows = sqlx::query_as::<_, ServerRow>(
            "SELECT id, name, host, port, username, ssh_key_encrypted, is_local, status, last_seen_at, created_at, updated_at
             FROM servers ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(
        &self,
        id: &str,
        name: &str,
        host: &str,
        port: u16,
        username: &str,
        ssh_key_encrypted: Option<&str>,
        is_local: bool,
    ) -> Result<Server> {
        let now = chrono::Utc::now().to_rfc3339();
        let is_local_int = if is_local { 1 } else { 0 };

        sqlx::query(
            "UPDATE servers
             SET name = ?, host = ?, port = ?, username = ?, ssh_key_encrypted = ?, is_local = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(name)
        .bind(host)
        .bind(port as i64)
        .bind(username)
        .bind(ssh_key_encrypted)
        .bind(is_local_int)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id).await?
            .ok_or_else(|| anyhow::anyhow!("Server not found"))
    }

    pub async fn update_status(&self, id: &str, status: ServerStatus, last_seen_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let status_str = status.as_str();
        let last_seen_str = last_seen_at.to_rfc3339();

        sqlx::query(
            "UPDATE servers
             SET status = ?, last_seen_at = ?
             WHERE id = ?"
        )
        .bind(status_str)
        .bind(&last_seen_str)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn find_local(&self) -> Result<Option<Server>> {
        let row = sqlx::query_as::<_, ServerRow>(
            "SELECT id, name, host, port, username, ssh_key_encrypted, is_local, status, last_seen_at, created_at, updated_at
             FROM servers WHERE is_local = 1 LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct ServerRow {
    id: String,
    name: String,
    host: String,
    port: i64,
    username: String,
    ssh_key_encrypted: Option<String>,
    is_local: i64,
    status: String,
    last_seen_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<ServerRow> for Server {
    fn from(row: ServerRow) -> Self {
        Server {
            id: row.id,
            name: row.name,
            host: row.host,
            port: row.port as u16,
            username: row.username,
            ssh_key_encrypted: row.ssh_key_encrypted,
            is_local: row.is_local != 0,
            status: ServerStatus::from_str(&row.status),
            last_seen_at: row.last_seen_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }
}
