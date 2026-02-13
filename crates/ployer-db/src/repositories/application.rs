use anyhow::Result;
use ployer_core::models::{Application, AppStatus, BuildStrategy};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ApplicationRepository {
    pool: SqlitePool,
}

impl ApplicationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        server_id: &str,
        git_url: Option<&str>,
        git_branch: &str,
        build_strategy: BuildStrategy,
        dockerfile_path: Option<&str>,
        port: Option<u16>,
        auto_deploy: bool,
    ) -> Result<Application> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let status = AppStatus::Idle.as_str();
        let strategy = build_strategy.as_str();

        sqlx::query(
            "INSERT INTO applications (id, name, server_id, git_url, git_branch, build_strategy, dockerfile_path, port, status, auto_deploy, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(name)
        .bind(server_id)
        .bind(git_url)
        .bind(git_branch)
        .bind(strategy)
        .bind(dockerfile_path)
        .bind(port.map(|p| p as i64))
        .bind(status)
        .bind(if auto_deploy { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.find_by_id(&id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created application"))
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Application>> {
        let row = sqlx::query_as::<_, ApplicationRow>(
            "SELECT id, name, server_id, git_url, git_branch, build_strategy, dockerfile_path, port, status, auto_deploy, created_at, updated_at
             FROM applications WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list(&self) -> Result<Vec<Application>> {
        let rows = sqlx::query_as::<_, ApplicationRow>(
            "SELECT id, name, server_id, git_url, git_branch, build_strategy, dockerfile_path, port, status, auto_deploy, created_at, updated_at
             FROM applications ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_by_server(&self, server_id: &str) -> Result<Vec<Application>> {
        let rows = sqlx::query_as::<_, ApplicationRow>(
            "SELECT id, name, server_id, git_url, git_branch, build_strategy, dockerfile_path, port, status, auto_deploy, created_at, updated_at
             FROM applications WHERE server_id = ? ORDER BY created_at DESC"
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(
        &self,
        id: &str,
        name: &str,
        git_url: Option<&str>,
        git_branch: &str,
        build_strategy: BuildStrategy,
        dockerfile_path: Option<&str>,
        port: Option<u16>,
        auto_deploy: bool,
    ) -> Result<Application> {
        let now = chrono::Utc::now().to_rfc3339();
        let strategy = build_strategy.as_str();

        sqlx::query(
            "UPDATE applications
             SET name = ?, git_url = ?, git_branch = ?, build_strategy = ?, dockerfile_path = ?, port = ?, auto_deploy = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(name)
        .bind(git_url)
        .bind(git_branch)
        .bind(strategy)
        .bind(dockerfile_path)
        .bind(port.map(|p| p as i64))
        .bind(if auto_deploy { 1 } else { 0 })
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id).await?
            .ok_or_else(|| anyhow::anyhow!("Application not found"))
    }

    pub async fn update_status(&self, id: &str, status: AppStatus) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let status_str = status.as_str();

        sqlx::query(
            "UPDATE applications
             SET status = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(status_str)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM applications WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ApplicationRow {
    id: String,
    name: String,
    server_id: String,
    git_url: Option<String>,
    git_branch: String,
    build_strategy: String,
    dockerfile_path: Option<String>,
    port: Option<i64>,
    status: String,
    auto_deploy: i64,
    created_at: String,
    updated_at: String,
}

impl From<ApplicationRow> for Application {
    fn from(row: ApplicationRow) -> Self {
        Application {
            id: row.id,
            name: row.name,
            server_id: row.server_id,
            git_url: row.git_url,
            git_branch: row.git_branch,
            build_strategy: BuildStrategy::from_str(&row.build_strategy),
            dockerfile_path: row.dockerfile_path,
            port: row.port.map(|p| p as u16),
            status: AppStatus::from_str(&row.status),
            auto_deploy: row.auto_deploy != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }
}
