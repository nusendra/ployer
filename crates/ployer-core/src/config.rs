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
