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
