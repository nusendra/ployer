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
fn caddy_client_admin_url() {
    let (dir, client) = make_client("admin-url");
    assert_eq!(client.admin_url(), "http://localhost:2019");
    let _ = fs::remove_dir_all(dir);
}
