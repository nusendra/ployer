//! Minimal Cloudflare DNS API client.
//!
//! Ployer already stores a Cloudflare API token for DNS-01 wildcard
//! certificates (see [`crate::CaddyClient`]). The same token — scoped
//! `Zone:DNS:Edit` — is enough to create the `A` records a domain needs, so
//! this module lets Ployer point a domain at the server itself instead of
//! asking the user to add the record by hand in the Cloudflare dashboard.
//!
//! Only the handful of endpoints Ployer needs are implemented: zone lookup and
//! `A`-record upsert/delete.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use tracing::{info, warn};

const API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// A Cloudflare zone (a domain whose DNS Cloudflare serves).
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct DnsRecord {
    id: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    proxied: Option<bool>,
}

/// Cloudflare wraps every response in a success/errors envelope; a 200 with
/// `success: false` is still a failure.
#[derive(Debug, Deserialize)]
struct Envelope<T> {
    success: bool,
    #[serde(default)]
    errors: Vec<ApiError>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: i64,
    message: String,
}

impl<T> Envelope<T> {
    fn into_result(self, what: &str) -> Result<T> {
        if !self.success {
            let detail = self
                .errors
                .iter()
                .map(|e| format!("{} ({})", e.message, e.code))
                .collect::<Vec<_>>()
                .join("; ");
            let detail = if detail.is_empty() { "unknown error".to_string() } else { detail };
            return Err(anyhow!("Cloudflare {} failed: {}", what, detail));
        }
        self.result
            .ok_or_else(|| anyhow!("Cloudflare {} returned no result", what))
    }
}

/// What an `A`-record upsert actually did — surfaced to the UI so the user can
/// tell "we created it for you" from "it was already right".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordOutcome {
    Created,
    Updated,
    Unchanged,
}

pub struct CloudflareClient {
    token: String,
    client: reqwest::Client,
}

impl CloudflareClient {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            client: reqwest::Client::new(),
        }
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{}", API_BASE, path))
            .bearer_auth(&self.token)
    }

    /// Verify the token is live and usable. Cheap pre-flight before saving a
    /// token or attempting a record write.
    pub async fn verify_token(&self) -> Result<bool> {
        let resp = self.get("/user/tokens/verify").send().await?;
        let env: Envelope<serde_json::Value> = resp.json().await?;
        Ok(env.success)
    }

    /// Every zone the token can see. Used to offer a picker and to resolve
    /// which zone a hostname belongs to.
    pub async fn list_zones(&self) -> Result<Vec<Zone>> {
        let resp = self.get("/zones?per_page=50&status=active").send().await?;
        let env: Envelope<Vec<Zone>> = resp.json().await?;
        env.into_result("zone list")
    }

    /// The zone that owns `hostname`, picked by longest matching suffix so
    /// `a.b.example.com` prefers a `b.example.com` zone over `example.com`.
    pub async fn find_zone(&self, hostname: &str) -> Result<Option<Zone>> {
        let host = hostname.trim_end_matches('.').to_lowercase();
        let mut best: Option<Zone> = None;
        for zone in self.list_zones().await? {
            let name = zone.name.to_lowercase();
            let owns = host == name || host.ends_with(&format!(".{}", name));
            if !owns {
                continue;
            }
            if best.as_ref().is_none_or(|b| b.name.len() < name.len()) {
                best = Some(zone);
            }
        }
        Ok(best)
    }

    async fn find_a_record(&self, zone_id: &str, fqdn: &str) -> Result<Option<DnsRecord>> {
        let resp = self
            .get(&format!("/zones/{}/dns_records?type=A&name={}", zone_id, fqdn))
            .send()
            .await?;
        let env: Envelope<Vec<DnsRecord>> = resp.json().await?;
        Ok(env.into_result("DNS record lookup")?.into_iter().next())
    }

    /// Create or correct the `A` record for `fqdn` so it resolves to `ip`.
    ///
    /// Records are written **DNS-only** (`proxied: false`): Caddy terminates
    /// TLS on the server, so routing the hostname through Cloudflare's proxy
    /// would break both the HTTP-01 challenge and the DNS-01 wildcard flow this
    /// token also serves.
    pub async fn upsert_a_record(&self, zone_id: &str, fqdn: &str, ip: &str) -> Result<RecordOutcome> {
        let body = serde_json::json!({
            "type": "A",
            "name": fqdn,
            "content": ip,
            "ttl": 1,          // 1 = "automatic"
            "proxied": false,
        });

        if let Some(existing) = self.find_a_record(zone_id, fqdn).await? {
            if existing.content == ip && existing.proxied == Some(false) {
                return Ok(RecordOutcome::Unchanged);
            }
            let resp = self
                .client
                .put(format!("{}/zones/{}/dns_records/{}", API_BASE, zone_id, existing.id))
                .bearer_auth(&self.token)
                .json(&body)
                .send()
                .await?;
            let env: Envelope<serde_json::Value> = resp.json().await?;
            env.into_result("DNS record update")?;
            info!("Cloudflare A record updated: {} -> {}", fqdn, ip);
            return Ok(RecordOutcome::Updated);
        }

        let resp = self
            .client
            .post(format!("{}/zones/{}/dns_records", API_BASE, zone_id))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;
        let env: Envelope<serde_json::Value> = resp.json().await?;
        env.into_result("DNS record create")?;
        info!("Cloudflare A record created: {} -> {}", fqdn, ip);
        Ok(RecordOutcome::Created)
    }

    /// Remove the `A` record for `fqdn`, if one exists. Missing is not an error
    /// — the desired end state (no record) already holds.
    pub async fn delete_a_record(&self, zone_id: &str, fqdn: &str) -> Result<bool> {
        let Some(existing) = self.find_a_record(zone_id, fqdn).await? else {
            return Ok(false);
        };
        let resp = self
            .client
            .delete(format!("{}/zones/{}/dns_records/{}", API_BASE, zone_id, existing.id))
            .bearer_auth(&self.token)
            .send()
            .await?;
        let env: Envelope<serde_json::Value> = resp.json().await?;
        if let Err(e) = env.into_result("DNS record delete") {
            warn!("{}", e);
            return Err(e);
        }
        Ok(true)
    }
}

/// Detect the server's public IPv4 address, trying several echo services in
/// turn. Returns `None` when every probe fails (offline, or egress blocked) —
/// callers fall back to a user-supplied IP.
pub async fn detect_public_ip() -> Option<String> {
    const PROBES: [&str; 3] = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
        "https://ipecho.net/plain",
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .ok()?;

    for url in PROBES {
        let Ok(resp) = client.get(url).send().await else { continue };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(body) = resp.text().await else { continue };
        let candidate = body.trim().to_string();
        if is_ipv4(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// A dotted-quad IPv4 literal. Kept local so callers don't need to parse the
/// echo services' plain-text bodies themselves.
pub fn is_ipv4(s: &str) -> bool {
    s.parse::<std::net::Ipv4Addr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::is_ipv4;

    #[test]
    fn ipv4_literals() {
        assert!(is_ipv4("3.144.143.144"));
        assert!(!is_ipv4("not-an-ip"));
        assert!(!is_ipv4("2001:db8::1"));
        assert!(!is_ipv4(""));
    }
}
