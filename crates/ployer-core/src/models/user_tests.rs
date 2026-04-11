use super::*;

#[test]
fn user_role_round_trips() {
    assert_eq!(UserRole::Admin.as_str(), "admin");
    assert_eq!(UserRole::User.as_str(), "user");
    assert_eq!(UserRole::from_str("admin"), UserRole::Admin);
    assert_eq!(UserRole::from_str("user"), UserRole::User);
}

#[test]
fn user_role_unknown_defaults_to_user() {
    assert_eq!(UserRole::from_str("superadmin"), UserRole::User);
}
