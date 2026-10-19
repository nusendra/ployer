//! Boot-time reconcile of Caddy upstreams against live container ports.
//!
//! App containers are published on **ephemeral** Docker host ports (an empty
//! `HostPort` so Docker picks a free port), which change on every container
//! start. The generated `apps.caddy` records `localhost:<host_port>` at deploy
//! time. After a host reboot the containers restart on *new* random ports, but
//! nothing rewrites `apps.caddy`, so Caddy keeps proxying the dead port and
//! every request returns 502 Bad Gateway (see INC-2026-08-20-slw-homes-502).
//!
//! This runs once at startup — alongside `reconcile_restart_policies` — and
//! re-points each managed app's routes at its container's *current* host port.
//! It is idempotent: a route already pointing at the live port is left
//! untouched (no rewrite, no Caddy reload), so a healthy host is a near no-op.

use ployer_core::models::Application;
use ployer_db::repositories::{ApplicationRepository, DomainRepository, SettingsRepository};
use ployer_docker::DockerClient;
use ployer_proxy::{CaddyClient, RouteSpec, DEFAULT_ROUTE_KEY};
use sqlx::SqlitePool;
use tracing::{info, warn};

/// Re-point every managed app's Caddy routes at its live container host port.
///
/// Returns the number of routes rewritten (drifted routes that were repointed).
/// Errors are confined to individual apps/routes and logged; the overall pass
/// only fails if the application list can't be read.
pub async fn reconcile_caddy_upstreams(
    db: &SqlitePool,
    docker: &DockerClient,
    caddy: &CaddyClient,
    base_domain: &str,
) -> anyhow::Result<usize> {
    let app_repo = ApplicationRepository::new(db.clone());
    let domain_repo = DomainRepository::new(db.clone());

    let apps = app_repo.list().await?;
    let default_app_id = SettingsRepository::new(db.clone()).default_app_id().await.ok().flatten();
    let mut fixed = 0usize;

    for app in apps {
        // No port => no HTTP route to reconcile (e.g. worker with no EXPOSE).
        let Some(port) = app.port else { continue };

        let container = format!("ployer-{}", app.slug());

        // Read the port Docker currently publishes for this container. A stopped
        // or missing container has no binding — skip it (can't route a dead app).
        let host_port = match docker.get_published_host_port(&container, port).await {
            Ok(Some(p)) => p,
            Ok(None) => continue,
            Err(e) => {
                warn!("Caddy reconcile: could not read host port for '{}': {}", container, e);
                continue;
            }
        };
        let upstream = format!("localhost:{}", host_port);

        // The auto subdomain (non-wildcard) plus every registered custom domain.
        let subdomain = format!("{}.{}", app.slug(), base_domain);
        let mut routes: Vec<(String, bool)> = vec![(subdomain.clone(), false)];
        match domain_repo.list_by_application(&app.id).await {
            Ok(domains) => {
                for d in domains {
                    if d.domain == subdomain {
                        continue;
                    }
                    routes.push((d.domain, d.wildcard));
                }
            }
            Err(e) => warn!("Caddy reconcile: could not list domains for '{}': {}", app.slug(), e),
        }

        for (domain, wildcard) in routes {
            // Skip routes already pointing at the live port — no needless reload.
            if caddy.current_upstream(&domain).as_deref() == Some(upstream.as_str()) {
                continue;
            }
            let spec = RouteSpec {
                domain: domain.clone(),
                upstream: upstream.clone(),
                wildcard,
                tls: caddy.tls_mode_for(&domain),
            };
            match caddy.persist_route_spec(&spec) {
                Ok(_) => {
                    info!("Caddy reconcile: repointed {} -> {}", domain, upstream);
                    fixed += 1;
                }
                Err(e) => warn!("Caddy reconcile: failed to repoint {}: {}", domain, e),
            }
        }

        // The default-app catch-all follows the same container port.
        if default_app_id.as_deref() == Some(app.id.as_str())
            && caddy.current_upstream(DEFAULT_ROUTE_KEY).as_deref() != Some(upstream.as_str())
        {
            match caddy.persist_default_route(&upstream) {
                Ok(_) => {
                    info!("Caddy reconcile: repointed default app -> {}", upstream);
                    fixed += 1;
                }
                Err(e) => warn!("Caddy reconcile: failed to repoint default app: {}", e),
            }
        }
    }

    Ok(fixed)
}

/// `localhost:<host port>` for `app`'s running container — what Caddy proxies
/// to. `None` when the app has no HTTP port or its container isn't running.
pub async fn live_upstream(docker: &DockerClient, app: &Application) -> Option<String> {
    let port = app.port?;
    let container = format!("ployer-{}", app.slug());
    match docker.get_published_host_port(&container, port).await {
        Ok(Some(host_port)) => Some(format!("localhost:{}", host_port)),
        Ok(None) => None,
        Err(e) => {
            warn!("Could not read host port for '{}': {}", container, e);
            None
        }
    }
}

/// Write `domain`'s route for `app` right away if its container is running,
/// so a newly added (or re-flagged) domain works without a redeploy. Returns
/// whether a route was written; otherwise the next deploy writes it.
pub async fn apply_domain_route(
    docker: Option<&DockerClient>,
    caddy: &CaddyClient,
    app: &Application,
    domain: &str,
    wildcard: bool,
) -> bool {
    let Some(docker) = docker else { return false };
    let Some(upstream) = live_upstream(docker, app).await else { return false };
    let spec = RouteSpec {
        domain: domain.to_string(),
        upstream,
        wildcard,
        tls: caddy.tls_mode_for(domain),
    };
    match caddy.persist_route_spec(&spec) {
        Ok(_) => true,
        Err(e) => {
            warn!("Failed to persist Caddy route for {}: {}", domain, e);
            false
        }
    }
}
