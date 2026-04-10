use axum::http::StatusCode;

type ValidationResult = Result<(), (StatusCode, String)>;

fn err(msg: &str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

/// Ensure a string field is non-empty and within max length.
pub fn required(value: &str, field: &str, max_len: usize) -> ValidationResult {
    if value.trim().is_empty() {
        return Err(err(&format!("{} is required", field)));
    }
    if value.len() > max_len {
        return Err(err(&format!("{} must be {} characters or fewer", field, max_len)));
    }
    Ok(())
}

/// Validate email format (basic check).
pub fn email(value: &str) -> ValidationResult {
    if value.trim().is_empty() {
        return Err(err("Email is required"));
    }
    if !value.contains('@') || !value.contains('.') {
        return Err(err("Invalid email address"));
    }
    if value.len() > 254 {
        return Err(err("Email must be 254 characters or fewer"));
    }
    Ok(())
}

/// Validate password strength.
pub fn password(value: &str) -> ValidationResult {
    if value.len() < 8 {
        return Err(err("Password must be at least 8 characters"));
    }
    if value.len() > 128 {
        return Err(err("Password must be 128 characters or fewer"));
    }
    Ok(())
}

/// Validate a git URL (must start with http/https/git@ or be empty).
pub fn git_url(value: &str) -> ValidationResult {
    let valid = value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("git@")
        || value.starts_with("ssh://");
    if !valid {
        return Err(err("git_url must start with http://, https://, git@, or ssh://"));
    }
    if value.len() > 2048 {
        return Err(err("git_url must be 2048 characters or fewer"));
    }
    Ok(())
}

/// Validate a TCP port number.
pub fn port(value: u16) -> ValidationResult {
    if value == 0 {
        return Err(err("Port must be between 1 and 65535"));
    }
    Ok(())
}

/// Validate an environment variable key (alphanumeric + underscore, no spaces).
pub fn env_key(value: &str) -> ValidationResult {
    if value.trim().is_empty() {
        return Err(err("Environment variable key is required"));
    }
    if value.len() > 256 {
        return Err(err("Environment variable key must be 256 characters or fewer"));
    }
    let valid = value.chars().all(|c| c.is_alphanumeric() || c == '_');
    if !valid {
        return Err(err("Environment variable key may only contain letters, digits, and underscores"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
