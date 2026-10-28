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

    /// Parse a `UserRole` out of the string stored in its database column.
    ///
    /// Total and lossy by design: an unrecognised value yields
    /// [`UserRole::User`] rather than an error, so a row written by a newer
    /// schema can never make an existing row unreadable.
    ///
    /// Anything that is not `"admin"` is treated as an unprivileged user, so a
    /// corrupt value fails closed.
    ///
    /// Deliberately not [`std::str::FromStr`]: that trait would need
    /// `Err = Infallible`, which hides the fallback from callers and makes
    /// `"nonsense".parse()` look like a successful parse.
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        }
    }
}

#[cfg(test)]
#[path = "user_tests.rs"]
mod tests;

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
