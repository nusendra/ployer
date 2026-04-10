pub mod repositories;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tracing::info;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    info!("Database connected: {}", database_url);
    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let migrations = [
        include_str!("../../../migrations/001_initial.sql"),
        include_str!("../../../migrations/002_webhooks.sql"),
        include_str!("../../../migrations/003_health_check_results.sql"),
        include_str!("../../../migrations/004_settings.sql"),
        include_str!("../../../migrations/005_resource_limits.sql"),
    ];

    for migration_sql in &migrations {
        for statement in migration_sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt).execute(pool).await?;
            }
        }
    }

    info!("Migrations applied successfully");
    Ok(())
}

#[cfg(test)]
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");
    run_migrations(&pool).await.expect("Migrations failed");
    pool
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        application::ApplicationRepository,
        user::UserRepository,
        server::ServerRepository,
        deployment::DeploymentRepository,
        env_var::EnvVarRepository,
        domain::DomainRepository,
        deploy_key::DeployKeyRepository,
    };
    use ployer_core::models::{AppStatus, BuildStrategy, DeploymentStatus, ServerStatus, UserRole};

    // ── User Repository ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn user_create_and_find_by_id() {
        let pool = test_pool().await;
        let repo = UserRepository::new(pool);
        let user = repo.create("test@example.com", "hash123", "Alice", UserRole::Admin).await.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, "Alice");

        let found = repo.find_by_id(&user.id).await.unwrap().unwrap();
        assert_eq!(found.id, user.id);
    }

    #[tokio::test]
    async fn user_find_by_email() {
        let pool = test_pool().await;
        let repo = UserRepository::new(pool);
        repo.create("findme@example.com", "hash", "Bob", UserRole::User).await.unwrap();

        let found = repo.find_by_email("findme@example.com").await.unwrap().unwrap();
        assert_eq!(found.email, "findme@example.com");
    }

    #[tokio::test]
    async fn user_find_by_email_nonexistent_returns_none() {
        let pool = test_pool().await;
        let repo = UserRepository::new(pool);
        let result = repo.find_by_email("nobody@example.com").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn user_count() {
        let pool = test_pool().await;
        let repo = UserRepository::new(pool);
        assert_eq!(repo.count().await.unwrap(), 0);
        repo.create("a@b.com", "hash", "A", UserRole::User).await.unwrap();
        assert_eq!(repo.count().await.unwrap(), 1);
        repo.create("b@b.com", "hash", "B", UserRole::User).await.unwrap();
        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn user_list() {
        let pool = test_pool().await;
        let repo = UserRepository::new(pool);
        repo.create("u1@test.com", "h", "U1", UserRole::User).await.unwrap();
        repo.create("u2@test.com", "h", "U2", UserRole::Admin).await.unwrap();
        let users = repo.list().await.unwrap();
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn user_update_password() {
        let pool = test_pool().await;
        let repo = UserRepository::new(pool);
        let user = repo.create("pw@test.com", "oldhash", "PW", UserRole::User).await.unwrap();
        repo.update_password(&user.id, "newhash").await.unwrap();
        let updated = repo.find_by_id(&user.id).await.unwrap().unwrap();
        assert_eq!(updated.password_hash, "newhash");
    }

    // ── Server Repository ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn server_create_and_find() {
        let pool = test_pool().await;
        let repo = ServerRepository::new(pool);
        let server = repo.create("My Server", "192.168.1.1", 22, "root", None, true).await.unwrap();
        assert_eq!(server.name, "My Server");
        assert_eq!(server.host, "192.168.1.1");
        assert!(server.is_local);

        let found = repo.find_by_id(&server.id).await.unwrap().unwrap();
        assert_eq!(found.id, server.id);
    }

    #[tokio::test]
    async fn server_list_and_delete() {
        let pool = test_pool().await;
        let repo = ServerRepository::new(pool);
        let s = repo.create("S1", "1.2.3.4", 22, "root", None, false).await.unwrap();
        assert_eq!(repo.list().await.unwrap().len(), 1);
        repo.delete(&s.id).await.unwrap();
        assert_eq!(repo.list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn server_update_status() {
        let pool = test_pool().await;
        let repo = ServerRepository::new(pool);
        let s = repo.create("S", "1.2.3.4", 22, "root", None, false).await.unwrap();
        repo.update_status(&s.id, ServerStatus::Online, chrono::Utc::now()).await.unwrap();
        let found = repo.find_by_id(&s.id).await.unwrap().unwrap();
        assert_eq!(found.status, ServerStatus::Online);
    }

    // ── Application Repository ────────────────────────────────────────────────

    async fn create_test_server(pool: SqlitePool) -> (ServerRepository, String) {
        let repo = ServerRepository::new(pool);
        let s = repo.create("TestServer", "127.0.0.1", 22, "root", None, true).await.unwrap();
        (repo, s.id)
    }

    #[tokio::test]
    async fn application_create_and_find() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let repo = ApplicationRepository::new(pool);

        let app = repo.create(
            "my-app", &server_id, Some("git@github.com:u/r.git"),
            "main", BuildStrategy::Dockerfile, None, Some(3000), false, None, None,
        ).await.unwrap();
        assert_eq!(app.name, "my-app");
        assert_eq!(app.port, Some(3000));

        let found = repo.find_by_id(&app.id).await.unwrap().unwrap();
        assert_eq!(found.id, app.id);
    }

    #[tokio::test]
    async fn application_update_port() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let repo = ApplicationRepository::new(pool);

        let app = repo.create(
            "port-app", &server_id, None, "main",
            BuildStrategy::Nixpacks, None, None, false, None, None,
        ).await.unwrap();
        assert_eq!(app.port, None);

        repo.update_port(&app.id, 8080).await.unwrap();
        let updated = repo.find_by_id(&app.id).await.unwrap().unwrap();
        assert_eq!(updated.port, Some(8080));
    }

    #[tokio::test]
    async fn application_update_status() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let repo = ApplicationRepository::new(pool);

        let app = repo.create(
            "status-app", &server_id, None, "main",
            BuildStrategy::Dockerfile, None, None, false, None, None,
        ).await.unwrap();

        repo.update_status(&app.id, AppStatus::Running).await.unwrap();
        let updated = repo.find_by_id(&app.id).await.unwrap().unwrap();
        assert_eq!(updated.status, AppStatus::Running);
    }

    #[tokio::test]
    async fn application_resource_limits() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let repo = ApplicationRepository::new(pool);

        let app = repo.create(
            "limits-app", &server_id, None, "main",
            BuildStrategy::Dockerfile, None, None, false, Some(0.5), Some(512),
        ).await.unwrap();
        assert_eq!(app.cpu_limit, Some(0.5));
        assert_eq!(app.memory_limit, Some(512));
    }

    #[tokio::test]
    async fn application_list_by_server() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let repo = ApplicationRepository::new(pool);

        repo.create("a1", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();
        repo.create("a2", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let apps = repo.list_by_server(&server_id).await.unwrap();
        assert_eq!(apps.len(), 2);
    }

    #[tokio::test]
    async fn application_delete() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let repo = ApplicationRepository::new(pool);

        let app = repo.create("del-app", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();
        repo.delete(&app.id).await.unwrap();
        assert!(repo.find_by_id(&app.id).await.unwrap().is_none());
    }

    // ── EnvVar Repository ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn env_var_create_and_list() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("env-app", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let env_repo = EnvVarRepository::new(pool);
        env_repo.create(&app.id, "DATABASE_URL", "encrypted-value").await.unwrap();
        env_repo.create(&app.id, "SECRET_KEY", "another-encrypted").await.unwrap();

        let vars = env_repo.list_by_application(&app.id).await.unwrap();
        assert_eq!(vars.len(), 2);
    }

    #[tokio::test]
    async fn env_var_find_by_key() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("env-app2", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let env_repo = EnvVarRepository::new(pool);
        env_repo.create(&app.id, "MY_KEY", "my-value").await.unwrap();

        let found = env_repo.find_by_application_and_key(&app.id, "MY_KEY").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().value_encrypted, "my-value");
    }

    #[tokio::test]
    async fn env_var_delete() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("env-app3", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let env_repo = EnvVarRepository::new(pool);
        env_repo.create(&app.id, "TO_DELETE", "val").await.unwrap();
        assert_eq!(env_repo.list_by_application(&app.id).await.unwrap().len(), 1);

        env_repo.delete(&app.id, "TO_DELETE").await.unwrap();
        assert_eq!(env_repo.list_by_application(&app.id).await.unwrap().len(), 0);
    }

    // ── Deployment Repository ─────────────────────────────────────────────────

    #[tokio::test]
    async fn deployment_create_and_update_status() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("dep-app", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let dep_repo = DeploymentRepository::new(pool);
        let dep = dep_repo.create(&app.id, &server_id, None, None, "ployer-dep-app:latest").await.unwrap();
        assert_eq!(dep.status, DeploymentStatus::Queued);

        dep_repo.update_status(&dep.id, DeploymentStatus::Running).await.unwrap();
        let updated = dep_repo.find_by_id(&dep.id).await.unwrap().unwrap();
        assert_eq!(updated.status, DeploymentStatus::Running);
    }

    #[tokio::test]
    async fn deployment_set_container_id() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("dep-app2", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let dep_repo = DeploymentRepository::new(pool);
        let dep = dep_repo.create(&app.id, &server_id, None, None, "image:latest").await.unwrap();
        dep_repo.set_container_id(&dep.id, "container-abc123").await.unwrap();

        let found = dep_repo.find_by_id(&dep.id).await.unwrap().unwrap();
        assert_eq!(found.container_id, Some("container-abc123".to_string()));
    }

    // ── Domain Repository ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn domain_create_and_find_by_domain() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("dom-app", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let domain_repo = DomainRepository::new(pool);
        domain_repo.create(&app.id, "myapp.example.com", true).await.unwrap();

        let found = domain_repo.find_by_domain("myapp.example.com").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().application_id, app.id);
    }

    #[tokio::test]
    async fn domain_list_by_application() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("dom-app2", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let domain_repo = DomainRepository::new(pool);
        domain_repo.create(&app.id, "app1.example.com", true).await.unwrap();
        domain_repo.create(&app.id, "app2.example.com", false).await.unwrap();

        let domains = domain_repo.list_by_application(&app.id).await.unwrap();
        assert_eq!(domains.len(), 2);
    }

    // ── DeployKey Repository ──────────────────────────────────────────────────

    #[tokio::test]
    async fn deploy_key_create_and_find() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("key-app", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let key_repo = DeployKeyRepository::new(pool);
        key_repo.create(&app.id, "ssh-rsa AAAA...", "encrypted-private").await.unwrap();

        let found = key_repo.find_by_application(&app.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().public_key, "ssh-rsa AAAA...");
    }

    #[tokio::test]
    async fn deploy_key_delete() {
        let pool = test_pool().await;
        let (_, server_id) = create_test_server(pool.clone()).await;
        let app_repo = ApplicationRepository::new(pool.clone());
        let app = app_repo.create("key-app2", &server_id, None, "main", BuildStrategy::Dockerfile, None, None, false, None, None).await.unwrap();

        let key_repo = DeployKeyRepository::new(pool);
        key_repo.create(&app.id, "ssh-rsa BBBB...", "encrypted").await.unwrap();
        assert!(key_repo.find_by_application(&app.id).await.unwrap().is_some());

        key_repo.delete(&app.id).await.unwrap();
        assert!(key_repo.find_by_application(&app.id).await.unwrap().is_none());
    }
}
