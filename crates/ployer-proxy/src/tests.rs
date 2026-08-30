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
    // Exactly one block for the domain (one marker).
    assert_eq!(content.matches("# ployer-route: app.example.com").count(), 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn persist_route_updates_upstream_on_redeploy() {
    let (dir, client) = make_client("update-upstream");

    // First deploy assigns one ephemeral port, a redeploy assigns another.
    client.persist_route("app.example.com", "localhost:32768").unwrap();
    client.persist_route("app.example.com", "localhost:32771").unwrap();

    let content = fs::read_to_string(client.apps_caddyfile_path()).unwrap();
    // Stale upstream gone, current one present, domain still single.
    assert!(!content.contains("localhost:32768"), "stale port not removed:\n{}", content);
    assert!(content.contains("reverse_proxy localhost:32771"));
    assert_eq!(content.matches("# ployer-route: app.example.com").count(), 1);
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
fn wildcard_cloudflare_block_has_tls_and_apex() {
    let (dir, client) = make_client("wildcard-cf");

    let spec = RouteSpec {
        domain: "slw.homes".to_string(),
        upstream: "localhost:32772".to_string(),
        wildcard: true,
        tls: TlsMode::CloudflareDns,
    };
    client.persist_route_spec(&spec).unwrap();

    let content = fs::read_to_string(client.apps_caddyfile_path()).unwrap();
    assert!(content.contains("*.slw.homes, slw.homes {"), "header:\n{}", content);
    assert!(content.contains("dns cloudflare {env.CF_API_TOKEN}"));
    assert!(content.contains("reverse_proxy localhost:32772"));
    // No http:// prefix for TLS routes.
    assert!(!content.contains("http://slw.homes"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn wildcard_block_uses_literal_token_when_set() {
    let (dir, client) = make_client("wildcard-literal");
    let client = client.with_cf_token(Some("cf-secret-123".to_string()));

    let spec = RouteSpec {
        domain: "slw.homes".to_string(),
        upstream: "localhost:32772".to_string(),
        wildcard: true,
        tls: TlsMode::CloudflareDns,
    };
    client.persist_route_spec(&spec).unwrap();

    let content = fs::read_to_string(client.apps_caddyfile_path()).unwrap();
    assert!(content.contains("dns cloudflare cf-secret-123"), "content:\n{}", content);
    assert!(!content.contains("{env.CF_API_TOKEN}"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn tls_mode_for_picks_http_for_nip_io_and_cf_for_real_domain() {
    let (dir, client) = make_client("tls-mode");
    let client = client.with_cf_token(Some("tok".to_string()));

    assert_eq!(client.tls_mode_for("app.1.2.3.4.nip.io"), TlsMode::Http);
    assert_eq!(client.tls_mode_for("slw.homes"), TlsMode::CloudflareDns);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn tls_mode_for_falls_back_to_http_without_token() {
    let (dir, client) = make_client("tls-mode-notoken");
    assert_eq!(client.tls_mode_for("slw.homes"), TlsMode::Http);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn upsert_replaces_legacy_block() {
    let (dir, client) = make_client("legacy-upsert");
    // Simulate a block written by an older ployer version (no marker).
    let legacy = "\nhttp://slw.homes {\n    reverse_proxy localhost:11111\n}\n";
    fs::write(client.apps_caddyfile_path(), legacy).unwrap();

    let spec = RouteSpec {
        domain: "slw.homes".to_string(),
        upstream: "localhost:22222".to_string(),
        wildcard: true,
        tls: TlsMode::CloudflareDns,
    };
    client.persist_route_spec(&spec).unwrap();

    let content = fs::read_to_string(client.apps_caddyfile_path()).unwrap();
    assert!(!content.contains("localhost:11111"), "legacy block not removed:\n{}", content);
    assert!(content.contains("reverse_proxy localhost:22222"));
    assert_eq!(content.matches("slw.homes {").count(), 1, "duplicate block:\n{}", content);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn current_upstream_reads_persisted_http_port() {
    let (dir, client) = make_client("current-upstream-http");
    client.persist_route("app.1.2.3.4.nip.io", "localhost:32768").unwrap();

    assert_eq!(
        client.current_upstream("app.1.2.3.4.nip.io").as_deref(),
        Some("localhost:32768")
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn current_upstream_reads_through_tls_block() {
    let (dir, client) = make_client("current-upstream-tls");
    let spec = RouteSpec {
        domain: "slw.homes".to_string(),
        upstream: "localhost:32768".to_string(),
        wildcard: true,
        tls: TlsMode::CloudflareDns,
    };
    client.persist_route_spec(&spec).unwrap();

    // Must skip the nested `tls { dns cloudflare ... }` directive and return the
    // reverse_proxy upstream — this is the exact INC-2026-08-20 shape.
    assert_eq!(
        client.current_upstream("slw.homes").as_deref(),
        Some("localhost:32768")
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn current_upstream_none_for_unknown_domain() {
    let (dir, client) = make_client("current-upstream-none");
    client.persist_route("app.example.com", "localhost:3000").unwrap();

    assert_eq!(client.current_upstream("other.example.com"), None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn current_upstream_tracks_drift_after_repoint() {
    let (dir, client) = make_client("current-upstream-drift");
    // Stale port from before a reboot, then repointed to the live port.
    client.persist_route("app.example.com", "localhost:32784").unwrap();
    client.persist_route("app.example.com", "localhost:32768").unwrap();

    assert_eq!(
        client.current_upstream("app.example.com").as_deref(),
        Some("localhost:32768")
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn caddy_client_admin_url() {
    let (dir, client) = make_client("admin-url");
    assert_eq!(client.admin_url(), "http://localhost:2019");
    let _ = fs::remove_dir_all(dir);
}

// ── Dashboard domain ────────────────────────────────────────────────

/// The installer's Caddyfile shape, for the "existing install" starting point.
fn write_installer_caddyfile(dir: &std::path::Path, domain: &str) {
    fs::write(
        dir.join("Caddyfile"),
        render_base_caddyfile(&[domain.to_string()], dir.join("apps.caddy").to_str().unwrap()),
    )
    .unwrap();
}

#[test]
fn dashboard_hosts_reads_the_site_block() {
    let (dir, client) = make_client("dashboard-hosts-read");
    write_installer_caddyfile(&dir, "3.144.143.144.nip.io");

    assert_eq!(client.dashboard_hosts(), vec!["3.144.143.144.nip.io"]);
    assert_eq!(client.dashboard_domain().as_deref(), Some("3.144.143.144.nip.io"));
}

#[test]
fn dashboard_hosts_ignores_catch_all_and_globals() {
    let (dir, client) = make_client("dashboard-hosts-ignores");
    // No named site at all: only the global block and the http:// catch-all.
    fs::write(
        dir.join("Caddyfile"),
        "{\n    auto_https disable_redirects\n}\n\nhttp:// {\n    reverse_proxy localhost:3001\n}\n",
    )
    .unwrap();

    assert!(client.dashboard_hosts().is_empty());
}

#[test]
fn set_dashboard_domain_keeps_nip_io_fallback() {
    let (dir, client) = make_client("dashboard-set-keeps-fallback");
    write_installer_caddyfile(&dir, "3.144.143.144.nip.io");

    client
        .set_dashboard_domain("ployer.example.com", &["3.144.143.144.nip.io".to_string()])
        .unwrap();

    let content = fs::read_to_string(dir.join("Caddyfile")).unwrap();
    assert!(content.contains("ployer.example.com, 3.144.143.144.nip.io {"));
    assert_eq!(
        client.dashboard_hosts(),
        vec!["ployer.example.com", "3.144.143.144.nip.io"]
    );
    // App routes must survive the rewrite.
    assert!(content.contains(&format!("import {}", dir.join("apps.caddy").display())));
}

#[test]
fn set_dashboard_domain_does_not_duplicate_the_new_host() {
    let (dir, client) = make_client("dashboard-set-no-dupes");
    write_installer_caddyfile(&dir, "ployer.example.com");

    client
        .set_dashboard_domain("ployer.example.com", &["Ployer.Example.com".to_string()])
        .unwrap();

    assert_eq!(client.dashboard_hosts(), vec!["ployer.example.com"]);
}

#[test]
fn set_dashboard_domain_backs_up_the_previous_caddyfile() {
    let (dir, client) = make_client("dashboard-set-backup");
    write_installer_caddyfile(&dir, "3.144.143.144.nip.io");

    client.set_dashboard_domain("ployer.example.com", &[]).unwrap();

    let backup = fs::read_to_string(dir.join("Caddyfile.bak")).unwrap();
    assert!(backup.contains("3.144.143.144.nip.io"));
}

#[test]
fn single_host_render_matches_the_installer_template() {
    let (dir, client) = make_client("dashboard-installer-parity");
    write_installer_caddyfile(&dir, "3.144.143.144.nip.io");
    let before = fs::read_to_string(dir.join("Caddyfile")).unwrap();

    // Re-setting the same domain with no extra hosts must be a no-op on content,
    // so a self-update (which rewrites this file) doesn't churn it.
    client.set_dashboard_domain("3.144.143.144.nip.io", &[]).unwrap();

    assert_eq!(before, fs::read_to_string(dir.join("Caddyfile")).unwrap());
}

#[test]
fn persist_env_rewrites_only_domain_keys() {
    let (dir, client) = make_client("dashboard-env-rewrite");
    fs::write(
        dir.join("ployer.env"),
        "PLOYER_HOST=0.0.0.0\n\
         PLOYER_BASE_DOMAIN=3.144.143.144.nip.io\n\
         PLOYER_PUBLIC_URL=https://3.144.143.144.nip.io\n\
         PLOYER_ALLOWED_ORIGINS=https://3.144.143.144.nip.io\n\
         PLOYER_JWT_SECRET=super-secret\n\
         CF_API_TOKEN=cf-token\n",
    )
    .unwrap();

    client.persist_dashboard_domain_env("ployer.example.com").unwrap();

    let env = fs::read_to_string(dir.join("ployer.env")).unwrap();
    assert!(env.contains("PLOYER_BASE_DOMAIN=ployer.example.com"));
    assert!(env.contains("PLOYER_PUBLIC_URL=https://ployer.example.com"));
    assert!(env.contains("PLOYER_ALLOWED_ORIGINS=https://ployer.example.com"));
    // Secrets untouched.
    assert!(env.contains("PLOYER_JWT_SECRET=super-secret"));
    assert!(env.contains("CF_API_TOKEN=cf-token"));
    assert!(env.contains("PLOYER_HOST=0.0.0.0"));
}

#[test]
fn persist_env_is_a_no_op_without_an_env_file() {
    let (dir, client) = make_client("dashboard-env-missing");
    let _ = fs::remove_file(dir.join("ployer.env"));

    client.persist_dashboard_domain_env("ployer.example.com").unwrap();

    assert!(!dir.join("ployer.env").exists());
}
