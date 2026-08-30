use anyhow::Result;
use sqlx::SqlitePool;

pub struct SettingsRepository {
    pool: SqlitePool,
}

impl SettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(v,)| v))
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn allow_registration(&self) -> Result<bool> {
        let val = self.get("allow_registration").await?.unwrap_or_else(|| "true".to_string());
        Ok(val == "true")
    }

    /// Cloudflare API token for HTTPS wildcard certs, if configured (non-empty).
    pub async fn cf_api_token(&self) -> Result<Option<String>> {
        Ok(self.get("cf_api_token").await?.filter(|s| !s.trim().is_empty()))
    }

    /// The server's public IPv4 address, as detected on boot or overridden by
    /// the user. Used as the target of the `A` records Ployer creates in
    /// Cloudflare, and shown in the UI for the manual path.
    pub async fn server_public_ip(&self) -> Result<Option<String>> {
        Ok(self.get("server_public_ip").await?.filter(|s| !s.trim().is_empty()))
    }

    pub async fn set_server_public_ip(&self, ip: &str) -> Result<()> {
        self.set("server_public_ip", ip).await
    }

    /// The custom dashboard domain configured through the UI, if any. Absent
    /// means the dashboard is still on the install-time `<ip>.nip.io` default.
    pub async fn dashboard_domain(&self) -> Result<Option<String>> {
        Ok(self.get("dashboard_domain").await?.filter(|s| !s.trim().is_empty()))
    }

    pub async fn set_dashboard_domain(&self, domain: &str) -> Result<()> {
        self.set("dashboard_domain", domain).await
    }
}
