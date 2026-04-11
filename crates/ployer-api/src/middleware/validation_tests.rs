use super::*;

// ── required ──────────────────────────────────────────────────────────────

#[test]
fn required_accepts_valid_value() {
    assert!(required("hello", "Field", 100).is_ok());
}

#[test]
fn required_rejects_empty_string() {
    assert!(required("", "Field", 100).is_err());
}

#[test]
fn required_rejects_whitespace_only() {
    assert!(required("   ", "Field", 100).is_err());
}

#[test]
fn required_rejects_value_exceeding_max_len() {
    let long = "a".repeat(101);
    assert!(required(&long, "Field", 100).is_err());
}

#[test]
fn required_accepts_value_at_exact_max_len() {
    let exact = "a".repeat(100);
    assert!(required(&exact, "Field", 100).is_ok());
}

// ── email ─────────────────────────────────────────────────────────────────

#[test]
fn email_accepts_valid_address() {
    assert!(email("user@example.com").is_ok());
}

#[test]
fn email_rejects_empty() {
    assert!(email("").is_err());
}

#[test]
fn email_rejects_missing_at_sign() {
    assert!(email("userexample.com").is_err());
}

#[test]
fn email_rejects_missing_dot() {
    assert!(email("user@examplecom").is_err());
}

#[test]
fn email_rejects_over_254_chars() {
    let long = format!("{}@b.com", "a".repeat(250));
    assert!(email(&long).is_err());
}

// ── password ──────────────────────────────────────────────────────────────

#[test]
fn password_accepts_valid() {
    assert!(password("securepass").is_ok());
}

#[test]
fn password_rejects_too_short() {
    assert!(password("short").is_err());
}

#[test]
fn password_accepts_exactly_8_chars() {
    assert!(password("exactly8").is_ok());
}

#[test]
fn password_rejects_over_128_chars() {
    let long = "a".repeat(129);
    assert!(password(&long).is_err());
}

#[test]
fn password_accepts_exactly_128_chars() {
    let max = "a".repeat(128);
    assert!(password(&max).is_ok());
}

// ── git_url ───────────────────────────────────────────────────────────────

#[test]
fn git_url_accepts_https() {
    assert!(git_url("https://github.com/user/repo.git").is_ok());
}

#[test]
fn git_url_accepts_http() {
    assert!(git_url("http://github.com/user/repo.git").is_ok());
}

#[test]
fn git_url_accepts_git_at() {
    assert!(git_url("git@github.com:user/repo.git").is_ok());
}

#[test]
fn git_url_accepts_ssh_scheme() {
    assert!(git_url("ssh://git@github.com/user/repo.git").is_ok());
}

#[test]
fn git_url_rejects_ftp_scheme() {
    assert!(git_url("ftp://github.com/user/repo.git").is_err());
}

#[test]
fn git_url_rejects_bare_path() {
    assert!(git_url("/some/local/path").is_err());
}

// ── port ──────────────────────────────────────────────────────────────────

#[test]
fn port_accepts_valid() {
    assert!(port(3000).is_ok());
}

#[test]
fn port_accepts_max() {
    assert!(port(65535).is_ok());
}

#[test]
fn port_rejects_zero() {
    assert!(port(0).is_err());
}

// ── env_key ───────────────────────────────────────────────────────────────

#[test]
fn env_key_accepts_valid() {
    assert!(env_key("DATABASE_URL").is_ok());
}

#[test]
fn env_key_accepts_with_digits() {
    assert!(env_key("VAR_123").is_ok());
}

#[test]
fn env_key_rejects_empty() {
    assert!(env_key("").is_err());
}

#[test]
fn env_key_rejects_hyphen() {
    assert!(env_key("MY-VAR").is_err());
}

#[test]
fn env_key_rejects_spaces() {
    assert!(env_key("MY VAR").is_err());
}

#[test]
fn env_key_rejects_over_256_chars() {
    let long = "A".repeat(257);
    assert!(env_key(&long).is_err());
}
