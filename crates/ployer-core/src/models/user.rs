use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl UserRole {
    pub fn as_str(&self) -> &str {
        match self {
            UserRole::Admin => "admin",
            UserRole::User => "user",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        }
    }
}

#[cfg(test)]
mod tests {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
