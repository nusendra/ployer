use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Clone)]
pub struct CaddyClient {
    admin_url: String,
    client: reqwest::Client,
    caddyfile_path: PathBuf,
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
    pub fn new(admin_url: &str, caddyfile_path: &str) -> Self {
        info!("Caddy client configured for {}", admin_url);
        Self {
            admin_url: admin_url.to_string(),
            client: reqwest::Client::new(),
            caddyfile_path: PathBuf::from(caddyfile_path),
        }
    }

    fn apps_caddyfile(&self) -> PathBuf {
        self.caddyfile_path
            .parent()
            .unwrap_or(Path::new("/opt/ployer"))
            .join("apps.caddy")
    }

    /// Write the app route to apps.caddy for persistence across restarts,
    /// then reload Caddy so the route takes effect immediately.
    pub fn persist_route(&self, domain: &str, upstream: &str) -> Result<()> {
        let apps_file = self.apps_caddyfile();

        // Read existing content
        let existing = std::fs::read_to_string(&apps_file).unwrap_or_default();

        // Only append if this domain isn't already in the file
        if !existing.contains(domain) {
            // Use http:// prefix to avoid Let's Encrypt rate-limit issues on shared
            // wildcard DNS services (nip.io, sslip.io). The main dashboard domain
            // keeps HTTPS; app subdomains are served over plain HTTP.
            let block = format!(
                "\nhttp://{} {{\n    reverse_proxy {}\n}}\n",
                domain, upstream
            );
            std::fs::write(&apps_file, format!("{}{}", existing, block))?;
            info!("Persisted Caddy route for {} -> {}", domain, upstream);
        }

        // Reload Caddy to pick up the new config
        let status = std::process::Command::new("caddy")
            .args(["reload", "--config", self.caddyfile_path.to_str().unwrap_or("/opt/ployer/Caddyfile")])
            .status();

        match status {
            Ok(s) if s.success() => info!("Caddy reloaded successfully"),
            Ok(s) => warn!("Caddy reload exited with status {}", s),
            Err(e) => warn!("Failed to run caddy reload: {}", e),
        }

        Ok(())
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

    /// Exposed for testing: path to the apps.caddy file
    #[cfg(test)]
    pub fn apps_caddyfile_path(&self) -> PathBuf {
        self.apps_caddyfile()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temp dir unique to a test name, set up an empty apps.caddy,
    /// and return a CaddyClient pointed at it.
    fn make_client(test_name: &str) -> (std::path::PathBuf, CaddyClient) {
        let dir = std::env::temp_dir().join(format!("ployer-proxy-test-{}", test_name));
        fs::create_dir_all(&dir).unwrap();
        let caddyfile = dir.join("Caddyfile");
        fs::write(dir.join("apps.caddy"), "").unwrap();
        let client = CaddyClient::new("http://localhost:2019", caddyfile.to_str().unwrap());
        (dir, client)
    }

    #[test]
    fn persist_route_writes_http_block() {
        let (dir, client) = make_client("writes-http-block");

        client.persist_route("myapp.1.2.3.4.nip.io", "localhost:3000").unwrap();

        let content = fs::read_to_string(client.apps_caddyfile_path()).unwrap();
        assert!(content.contains("http://myapp.1.2.3.4.nip.io"));
        assert!(content.contains("reverse_proxy localhost:3000"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn persist_route_does_not_duplicate_existing_domain() {
        let (dir, client) = make_client("no-duplicate");

        client.persist_route("app.example.com", "localhost:8080").unwrap();
        client.persist_route("app.example.com", "localhost:8080").unwrap();

        let content = fs::read_to_string(client.apps_caddyfile_path()).unwrap();
        // Domain should appear only once
        assert_eq!(content.matches("app.example.com").count(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn persist_route_appends_multiple_different_domains() {
        let (dir, client) = make_client("multi-domains");

        client.persist_route("app1.example.com", "localhost:3001").unwrap();
        client.persist_route("app2.example.com", "localhost:3002").unwrap();

        let content = fs::read_to_string(client.apps_caddyfile_path()).unwrap();
        assert!(content.contains("app1.example.com"));
        assert!(content.contains("app2.example.com"));
        assert!(content.contains("localhost:3001"));
        assert!(content.contains("localhost:3002"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn caddy_client_admin_url() {
        let (dir, client) = make_client("admin-url");
        assert_eq!(client.admin_url(), "http://localhost:2019");
        let _ = fs::remove_dir_all(dir);
    }
}
