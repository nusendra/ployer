use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub docker: DockerConfig,
    pub caddy: CaddyConfig,
}

impl AppConfig {
    /// Derive a 32-byte encryption key from the JWT secret using SHA-256
    pub fn get_secret_key(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.auth.jwt_secret.as_bytes());
        hasher.finalize().into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_domain: String,
    pub public_url: String,
    /// Comma-separated list of allowed CORS origins, e.g. "http://localhost:5173,https://app.example.com"
    /// Use "*" to allow all origins (default, suitable for development).
    pub allowed_origins: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyConfig {
    pub admin_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3001,
                base_domain: "localhost".to_string(),
                public_url: "http://localhost:3001".to_string(),
                allowed_origins: "*".to_string(),
            },
            database: DatabaseConfig {
                url: "sqlite://ployer.db?mode=rwc".to_string(),
            },
            auth: AuthConfig {
                jwt_secret: "change-me-in-production".to_string(),
                token_expiry_hours: 24,
            },
            docker: DockerConfig {
                socket_path: "/var/run/docker.sock".to_string(),
            },
            caddy: CaddyConfig {
                admin_url: "http://localhost:2019".to_string(),
            },
        }
    }
}

impl AppConfig {
    /// Load config from environment variables, falling back to defaults.
    ///
    /// Supported env vars:
    ///   PLOYER_HOST, PLOYER_PORT, PLOYER_BASE_DOMAIN, PLOYER_PUBLIC_URL,
    ///   PLOYER_ALLOWED_ORIGINS, PLOYER_DATABASE_URL, PLOYER_JWT_SECRET,
    ///   PLOYER_TOKEN_EXPIRY_HOURS, PLOYER_DOCKER_SOCKET, PLOYER_CADDY_URL
    pub fn from_env() -> Self {
        let mut cfg = Self::default();

        if let Ok(v) = std::env::var("PLOYER_HOST")            { cfg.server.host = v; }
        if let Ok(v) = std::env::var("PLOYER_PORT")            { if let Ok(p) = v.parse() { cfg.server.port = p; } }
        if let Ok(v) = std::env::var("PLOYER_BASE_DOMAIN")     { cfg.server.base_domain = v; }
        if let Ok(v) = std::env::var("PLOYER_PUBLIC_URL")      { cfg.server.public_url = v; }
        if let Ok(v) = std::env::var("PLOYER_ALLOWED_ORIGINS") { cfg.server.allowed_origins = v; }
        if let Ok(v) = std::env::var("PLOYER_DATABASE_URL")    { cfg.database.url = v; }
        if let Ok(v) = std::env::var("PLOYER_JWT_SECRET")      { cfg.auth.jwt_secret = v; }
        if let Ok(v) = std::env::var("PLOYER_TOKEN_EXPIRY_HOURS") { if let Ok(h) = v.parse() { cfg.auth.token_expiry_hours = h; } }
        if let Ok(v) = std::env::var("PLOYER_DOCKER_SOCKET")   { cfg.docker.socket_path = v; }
        if let Ok(v) = std::env::var("PLOYER_CADDY_URL")       { cfg.caddy.admin_url = v; }

        cfg
    }
}
