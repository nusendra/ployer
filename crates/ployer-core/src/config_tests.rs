use super::*;

#[test]
fn default_config_has_expected_values() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.server.host, "0.0.0.0");
    assert_eq!(cfg.server.port, 3001);
    assert_eq!(cfg.server.base_domain, "localhost");
    assert_eq!(cfg.auth.jwt_secret, "change-me-in-production");
    assert_eq!(cfg.auth.token_expiry_hours, 24);
    assert_eq!(cfg.docker.socket_path, "/var/run/docker.sock");
    assert_eq!(cfg.caddy.admin_url, "http://localhost:2019");
}

#[test]
fn get_secret_key_returns_32_bytes() {
    let cfg = AppConfig::default();
    let key = cfg.get_secret_key();
    assert_eq!(key.len(), 32);
}

#[test]
fn get_secret_key_is_deterministic() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.get_secret_key(), cfg.get_secret_key());
}

#[test]
fn get_secret_key_differs_for_different_secrets() {
    let mut cfg1 = AppConfig::default();
    cfg1.auth.jwt_secret = "secret-one".to_string();
    let mut cfg2 = AppConfig::default();
    cfg2.auth.jwt_secret = "secret-two".to_string();
    assert_ne!(cfg1.get_secret_key(), cfg2.get_secret_key());
}

#[test]
fn from_env_falls_back_to_defaults_when_no_env_vars() {
    // Unset all ployer env vars to ensure defaults are used
    let vars = [
        "PLOYER_HOST", "PLOYER_PORT", "PLOYER_BASE_DOMAIN", "PLOYER_PUBLIC_URL",
        "PLOYER_ALLOWED_ORIGINS", "PLOYER_DATABASE_URL", "PLOYER_JWT_SECRET",
        "PLOYER_TOKEN_EXPIRY_HOURS", "PLOYER_DOCKER_SOCKET", "PLOYER_CADDY_URL", "PLOYER_CADDYFILE",
    ];
    for v in &vars { std::env::remove_var(v); }

    let cfg = AppConfig::from_env();
    let default = AppConfig::default();
    assert_eq!(cfg.server.host, default.server.host);
    assert_eq!(cfg.server.port, default.server.port);
    assert_eq!(cfg.auth.jwt_secret, default.auth.jwt_secret);
}
