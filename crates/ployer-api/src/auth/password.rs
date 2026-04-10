use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    Ok(password_hash)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let pw = "mysecretpassword";
        let hash = hash_password(pw).unwrap();
        assert!(verify_password(pw, &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails_verification() {
        let hash = hash_password("correctpassword").unwrap();
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }

    #[test]
    fn each_hash_is_unique() {
        let pw = "samepassword";
        let hash1 = hash_password(pw).unwrap();
        let hash2 = hash_password(pw).unwrap();
        // Same password produces different hashes (random salt)
        assert_ne!(hash1, hash2);
        // But both verify correctly
        assert!(verify_password(pw, &hash1).unwrap());
        assert!(verify_password(pw, &hash2).unwrap());
    }

    #[test]
    fn invalid_hash_string_returns_error() {
        let result = verify_password("password", "not-a-valid-hash");
        assert!(result.is_err());
    }
}
