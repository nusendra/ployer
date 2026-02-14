use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Clone)]
pub struct CaddyClient {
    admin_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
pub struct ReverseProxyConfig {
    pub domain: String,
    pub upstream: String, // e.g., "localhost:8080"
    pub enable_https: bool,
}

#[derive(Debug, Deserialize)]
pub struct RouteInfo {
    pub domain: String,
    pub upstream: String,
    pub ssl_status: String,
}

impl CaddyClient {
    pub fn new(admin_url: &str) -> Self {
        info!("Caddy client configured for {}", admin_url);
        Self {
            admin_url: admin_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn ping(&self) -> Result<bool> {
        match self.client.get(&self.admin_url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub fn admin_url(&self) -> &str {
        &self.admin_url
    }

    /// Add a reverse proxy route for a domain
    /// Caddy will automatically obtain SSL certificates via Let's Encrypt
    pub async fn add_route(&self, config: ReverseProxyConfig) -> Result<()> {
        info!("Adding Caddy route: {} -> {}", config.domain, config.upstream);

        // Build Caddy JSON config for reverse proxy
        let caddy_config = serde_json::json!({
            "match": [{
                "host": [config.domain]
            }],
            "handle": [{
                "handler": "reverse_proxy",
                "upstreams": [{
                    "dial": config.upstream
                }]
            }]
        });

        // POST to Caddy's config API
        let url = format!("{}/config/apps/http/servers/srv0/routes", self.admin_url);
        let resp = self.client
            .post(&url)
            .json(&caddy_config)
            .send()
            .await?;

        if resp.status().is_success() {
            info!("Caddy route added successfully for {}", config.domain);
            Ok(())
        } else {
            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            warn!("Failed to add Caddy route: {}", error_text);
            Err(anyhow!("Failed to add Caddy route: {}", error_text))
        }
    }

    /// Remove a route by domain
    pub async fn remove_route(&self, domain: &str) -> Result<()> {
        info!("Removing Caddy route for domain: {}", domain);

        // For simplicity, we'll reload the entire config without this domain
        // In production, you'd use Caddy's @id-based route removal
        warn!("Route removal is a stub - implement with Caddy route IDs in production");

        // TODO: Implement proper route removal using Caddy's route IDs
        // For now, just log the intention
        Ok(())
    }

    /// List all active routes (stub for now)
    pub async fn list_routes(&self) -> Result<Vec<RouteInfo>> {
        info!("Listing Caddy routes");

        // Get current Caddy config
        let url = format!("{}/config/apps/http/servers", self.admin_url);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch Caddy config"));
        }

        // For MVP, return empty list
        // TODO: Parse Caddy JSON config and extract routes
        Ok(Vec::new())
    }

    /// Get SSL certificate status for a domain
    pub async fn get_ssl_status(&self, domain: &str) -> Result<String> {
        info!("Checking SSL status for domain: {}", domain);

        // Query Caddy's certificate storage
        let url = format!("{}/config/apps/tls/certificates", self.admin_url);
        let resp = self.client.get(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                // For MVP, assume SSL is active if Caddy is running
                Ok("active".to_string())
            }
            _ => Ok("pending".to_string())
        }
    }
}
