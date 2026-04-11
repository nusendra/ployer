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
