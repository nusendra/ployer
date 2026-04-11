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
