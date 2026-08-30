pub mod cloudflare;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

#[derive(Clone)]
pub struct CaddyClient {
    admin_url: String,
    client: reqwest::Client,
    caddyfile_path: PathBuf,
    /// Cloudflare API token for DNS-01 wildcard certs. When present, real
    /// domains (not nip.io/sslip.io) are served over HTTPS with a Caddy
    /// `tls { dns cloudflare ... }` block instead of plain http://.
    ///
    /// Shared + interior-mutable so it can be updated at runtime (e.g. from the
    /// settings UI) and seen by every clone of the client.
    cf_api_token: Arc<RwLock<Option<String>>>,
}

#[derive(Debug, Serialize)]
pub struct ReverseProxyConfig {
    pub domain: String,
    pub upstream: String, // e.g., "localhost:8080"
    pub enable_https: bool,
}

/// How a route terminates TLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsMode {
    /// Plain HTTP (no cert). Used for shared wildcard-DNS services like
    /// nip.io/sslip.io where per-host Let's Encrypt hits rate limits.
    Http,
    /// HTTPS via Let's Encrypt DNS-01 using the Cloudflare provider. Works for
    /// wildcard certs (`*.example.com`) which HTTP-01 cannot issue.
    CloudflareDns,
}

/// A single Caddy route to persist.
#[derive(Debug, Clone)]
pub struct RouteSpec {
    pub domain: String,
    pub upstream: String,
    /// Also serve `*.<domain>` (tenant subdomains) alongside the apex.
    pub wildcard: bool,
    pub tls: TlsMode,
}

/// Render the apps.caddy block for a route. Each block is preceded by a
/// `# ployer-route: <domain>` marker so it can be upserted regardless of shape.
///
/// For Cloudflare TLS the token is written literally when known (so a
/// UI-configured token applies on a plain `caddy reload` without a service
/// restart); otherwise it falls back to `{env.CF_API_TOKEN}` for the
/// install-time env path.
fn render_block(spec: &RouteSpec, cf_token: Option<&str>) -> String {
    let mut hosts: Vec<String> = Vec::new();
    match spec.tls {
        TlsMode::Http => {
            // http:// prefix keeps Caddy from auto-upgrading to HTTPS.
            if spec.wildcard {
                hosts.push(format!("http://*.{}", spec.domain));
            }
            hosts.push(format!("http://{}", spec.domain));
        }
        TlsMode::CloudflareDns => {
            if spec.wildcard {
                hosts.push(format!("*.{}", spec.domain));
            }
            hosts.push(spec.domain.clone());
        }
    }

    let mut block = format!("# ployer-route: {}\n{} {{\n", spec.domain, hosts.join(", "));
    if spec.tls == TlsMode::CloudflareDns {
        let token = match cf_token {
            Some(t) if !t.is_empty() => t,
            _ => "{env.CF_API_TOKEN}",
        };
        block.push_str(&format!("    tls {{\n        dns cloudflare {}\n    }}\n", token));
    }
    block.push_str(&format!("    reverse_proxy {}\n}}\n", spec.upstream));
    block
}

/// Render the base Caddyfile: the dashboard site block, the plain-HTTP
/// LAN/direct-IP catch-all, and the import of the app routes file.
///
/// `hosts` are the dashboard's addresses, in order. More than one is the norm
/// after moving to a custom domain: the install-time `<ip>.nip.io` address is
/// kept alongside it so the dashboard stays reachable over HTTPS while the new
/// domain's DNS propagates (and if it never does).
///
/// With a single host this is byte-compatible with the template `install.sh`
/// writes, so a self-update — which re-runs the installer — produces the same
/// file for the same domain.
fn render_base_caddyfile(hosts: &[String], apps_caddy_path: &str) -> String {
    let domain = hosts.join(", ");
    format!(
        r#"{{
    # Disable Caddy's catch-all HTTP→HTTPS redirect so app subdomains
    # served with http:// prefix are not silently upgraded to HTTPS.
    auto_https disable_redirects
}}

{domain} {{
    reverse_proxy localhost:3001
}}

# LAN / direct-IP access (HTTP only — no cert possible for private IPs).
# Catches any host not matched above (e.g. http://192.168.x.x, http://hostname.local).
http:// {{
    reverse_proxy localhost:3001
}}

import {apps_caddy_path}
"#
    )
}

#[derive(Debug, Deserialize)]
pub struct RouteInfo {
    pub domain: String,
    pub upstream: String,
    pub ssl_status: String,
}

/// Remove any existing block for `domain` from an apps.caddy file, leaving all
/// other blocks intact. Used to upsert a route so the upstream can be refreshed
/// on redeploy.
///
/// Handles two shapes:
///   * Marker blocks written by the current code: a `# ployer-route: <domain>`
///     line followed by a block whose braces may nest (e.g. a `tls { ... }`
///     directive). Removal is brace-balanced.
///   * Legacy blocks written by older versions: a single-line `http://<domain> {`
///     opener closed by a lone `}` line, with no marker.
fn remove_domain_block(content: &str, domain: &str) -> String {
    let marker = format!("# ployer-route: {}", domain);
    let legacy_opener = format!("http://{} {{", domain);
    let mut out = String::new();
    let mut lines = content.lines();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Marker block: skip the marker and the following brace-balanced block.
        if trimmed == marker {
            let mut depth: i32 = 0;
            let mut opened = false;
            for bl in lines.by_ref() {
                for c in bl.chars() {
                    match c {
                        '{' => {
                            depth += 1;
                            opened = true;
                        }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                if opened && depth <= 0 {
                    break;
                }
            }
            continue;
        }

        // Legacy block: single-line opener closed by a lone `}`.
        if line.trim_start().starts_with(&legacy_opener) {
            for bl in lines.by_ref() {
                if bl.trim() == "}" {
                    break;
                }
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }
    out
}

impl CaddyClient {
    pub fn new(admin_url: &str, caddyfile_path: &str) -> Self {
        info!("Caddy client configured for {}", admin_url);
        Self {
            admin_url: admin_url.to_string(),
            client: reqwest::Client::new(),
            caddyfile_path: PathBuf::from(caddyfile_path),
            cf_api_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Attach a Cloudflare API token, enabling HTTPS (DNS-01) for real domains.
    pub fn with_cf_token(self, token: Option<String>) -> Self {
        self.set_cf_token(token);
        self
    }

    /// Update the Cloudflare token at runtime. Empty/blank clears it. Shared
    /// across all clones of this client.
    pub fn set_cf_token(&self, token: Option<String>) {
        let cleaned = token.filter(|t| !t.trim().is_empty());
        if let Ok(mut guard) = self.cf_api_token.write() {
            *guard = cleaned;
        }
    }

    /// Current token, if configured.
    pub fn cf_token(&self) -> Option<String> {
        self.cf_api_token.read().ok().and_then(|g| g.clone())
    }

    /// Whether a Cloudflare token is configured.
    pub fn cf_token_is_set(&self) -> bool {
        self.cf_token().is_some()
    }

    /// Whether the running Caddy binary has the Cloudflare DNS provider compiled
    /// in. Without it, HTTPS wildcard certs cannot be issued.
    pub fn cloudflare_plugin_available(&self) -> bool {
        match std::process::Command::new("caddy").arg("list-modules").output() {
            Ok(out) => String::from_utf8_lossy(&out.stdout).contains("dns.providers.cloudflare"),
            Err(_) => false,
        }
    }

    /// Choose the TLS mode for a domain. Shared wildcard-DNS services stay on
    /// plain HTTP (LE rate limits); real domains use Cloudflare DNS-01 when a
    /// token is configured, otherwise fall back to HTTP.
    pub fn tls_mode_for(&self, domain: &str) -> TlsMode {
        let d = domain.trim_end_matches('.');
        let shared_ip_dns = d.ends_with(".nip.io") || d.ends_with(".sslip.io");
        if !shared_ip_dns && self.cf_token_is_set() {
            TlsMode::CloudflareDns
        } else {
            TlsMode::Http
        }
    }

    fn apps_caddyfile(&self) -> PathBuf {
        self.caddyfile_path
            .parent()
            .unwrap_or(Path::new("/opt/ployer"))
            .join("apps.caddy")
    }

    /// Reload Caddy so config changes take effect immediately.
    fn reload(&self) {
        let status = std::process::Command::new("caddy")
            .args(["reload", "--config", self.caddyfile_path.to_str().unwrap_or("/opt/ployer/Caddyfile")])
            .status();

        match status {
            Ok(s) if s.success() => info!("Caddy reloaded successfully"),
            Ok(s) => warn!("Caddy reload exited with status {}", s),
            Err(e) => warn!("Failed to run caddy reload: {}", e),
        }
    }

    /// Upsert a route into apps.caddy and reload Caddy.
    ///
    /// Any existing block for this domain is removed and rewritten with the
    /// current upstream. App containers are published on ephemeral host ports
    /// that change on every deploy, so a stale route would otherwise point at a
    /// dead port and return 502 after a redeploy.
    pub fn persist_route_spec(&self, spec: &RouteSpec) -> Result<()> {
        let apps_file = self.apps_caddyfile();

        // Read existing content and drop any prior block for this domain.
        let existing = std::fs::read_to_string(&apps_file).unwrap_or_default();
        let filtered = remove_domain_block(&existing, &spec.domain);

        let block = render_block(spec, self.cf_token().as_deref());
        let mut content = filtered.trim_end().to_string();
        if !content.is_empty() {
            content.push('\n');
        }
        content.push('\n');
        content.push_str(&block);
        std::fs::write(&apps_file, content)?;
        info!(
            "Persisted Caddy route for {} -> {} (wildcard={}, tls={:?})",
            spec.domain, spec.upstream, spec.wildcard, spec.tls
        );

        self.reload();
        Ok(())
    }

    /// Read the `reverse_proxy` upstream currently persisted for `domain` in
    /// apps.caddy, if any.
    ///
    /// Used by the boot-time reconcile to detect a stale upstream — e.g. after
    /// a reboot reshuffled the container's ephemeral host port — so a route is
    /// only rewritten (and Caddy only reloaded) when the port actually drifted.
    /// Returns `None` when the file is missing, the domain has no block, or the
    /// block carries no `reverse_proxy` directive.
    pub fn current_upstream(&self, domain: &str) -> Option<String> {
        let content = std::fs::read_to_string(self.apps_caddyfile()).ok()?;
        let marker = format!("# ployer-route: {}", domain);
        let mut lines = content.lines();
        while let Some(line) = lines.next() {
            if line.trim() != marker {
                continue;
            }
            // Scan this brace-balanced block for its reverse_proxy directive.
            let mut depth: i32 = 0;
            let mut opened = false;
            for bl in lines.by_ref() {
                if let Some(rest) = bl.trim().strip_prefix("reverse_proxy ") {
                    return Some(rest.trim().to_string());
                }
                for c in bl.chars() {
                    match c {
                        '{' => {
                            depth += 1;
                            opened = true;
                        }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                if opened && depth <= 0 {
                    break;
                }
            }
            return None;
        }
        None
    }

    /// Convenience: persist a plain HTTP, non-wildcard route.
    pub fn persist_route(&self, domain: &str, upstream: &str) -> Result<()> {
        self.persist_route_spec(&RouteSpec {
            domain: domain.to_string(),
            upstream: upstream.to_string(),
            wildcard: false,
            tls: TlsMode::Http,
        })
    }

    // ── Dashboard domain ─────────────────────────────────────────────
    //
    // The dashboard's own hostname lives in the base Caddyfile (written by
    // install.sh) rather than in apps.caddy, so moving Ployer off the
    // install-time `<ip>.nip.io` default means rewriting that file and the
    // `ployer.env` values a restart would otherwise read back.

    /// Directory holding the Caddyfile — also where `ployer.env`, `apps.caddy`
    /// and the rest of the install live.
    fn ployer_dir(&self) -> PathBuf {
        self.caddyfile_path
            .parent()
            .unwrap_or(Path::new("/opt/ployer"))
            .to_path_buf()
    }

    fn env_file(&self) -> PathBuf {
        self.ployer_dir().join("ployer.env")
    }

    /// Every hostname the base Caddyfile currently serves the dashboard on, in
    /// file order. These come from the first top-level site block that isn't
    /// the `http://` LAN catch-all or the global-options block.
    pub fn dashboard_hosts(&self) -> Vec<String> {
        let Ok(content) = std::fs::read_to_string(&self.caddyfile_path) else {
            return Vec::new();
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            let Some(addr) = trimmed.strip_suffix('{') else { continue };
            let addr = addr.trim();
            if addr.is_empty() || addr == "http://" {
                continue;
            }
            return addr
                .split(',')
                .map(|h| h.trim().to_string())
                .filter(|h| !h.is_empty())
                .collect();
        }
        Vec::new()
    }

    /// The primary dashboard hostname currently served, if any.
    pub fn dashboard_domain(&self) -> Option<String> {
        self.dashboard_hosts().into_iter().next()
    }

    /// Point the dashboard at `domain` and reload Caddy.
    ///
    /// `keep_hosts` are additional addresses to keep serving the dashboard on —
    /// normally the install-time `<ip>.nip.io` fallback, so a domain whose DNS
    /// isn't live yet can't lock the user out of their own dashboard.
    ///
    /// The base Caddyfile is regenerated from the same template `install.sh`
    /// writes (which also overwrites it on every self-update), preserving the
    /// `import` of `apps.caddy` so deployed app routes are untouched.
    pub fn set_dashboard_domain(&self, domain: &str, keep_hosts: &[String]) -> Result<()> {
        let mut hosts = vec![domain.to_string()];
        for host in keep_hosts {
            let host = host.trim();
            if !host.is_empty() && !hosts.iter().any(|h| h.eq_ignore_ascii_case(host)) {
                hosts.push(host.to_string());
            }
        }

        let apps_import = self.apps_caddyfile();
        let content = render_base_caddyfile(&hosts, &apps_import.to_string_lossy());

        // Keep a copy of the file we are replacing so a bad domain can be
        // restored by hand without re-running the installer.
        if let Ok(previous) = std::fs::read_to_string(&self.caddyfile_path) {
            let _ = std::fs::write(self.caddyfile_path.with_extension("bak"), previous);
        }

        std::fs::write(&self.caddyfile_path, content)?;
        info!("Dashboard domain set to {}", domain);
        self.reload();
        Ok(())
    }

    /// Persist the dashboard domain into `ployer.env` so a restart — and the
    /// installer's "preserve existing domain" path on self-update — keep it.
    ///
    /// Only the three domain-derived keys are rewritten; every other line is
    /// left byte-for-byte intact so secrets are never reformatted.
    pub fn persist_dashboard_domain_env(&self, domain: &str) -> Result<()> {
        let path = self.env_file();
        let Ok(existing) = std::fs::read_to_string(&path) else {
            // No env file (dev runs, container installs) — nothing to persist.
            return Ok(());
        };

        let public_url = format!("https://{}", domain);
        let mut out = String::new();
        let mut seen_base = false;
        for line in existing.lines() {
            let replacement = match line.split_once('=').map(|(k, _)| k.trim()) {
                Some("PLOYER_BASE_DOMAIN") => {
                    seen_base = true;
                    Some(format!("PLOYER_BASE_DOMAIN={}", domain))
                }
                Some("PLOYER_PUBLIC_URL") => Some(format!("PLOYER_PUBLIC_URL={}", public_url)),
                Some("PLOYER_ALLOWED_ORIGINS") => {
                    Some(format!("PLOYER_ALLOWED_ORIGINS={}", public_url))
                }
                _ => None,
            };
            out.push_str(replacement.as_deref().unwrap_or(line));
            out.push('\n');
        }
        if !seen_base {
            out.push_str(&format!("PLOYER_BASE_DOMAIN={}\n", domain));
        }

        std::fs::write(&path, out)?;
        // The file carries the JWT secret and Cloudflare token.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
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
mod tests;
