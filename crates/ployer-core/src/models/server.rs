use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(skip_serializing)]
    pub ssh_key_encrypted: Option<String>,
    pub is_local: bool,
    pub status: ServerStatus,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Online,
    Offline,
    Unknown,
}

impl ServerStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ServerStatus::Online => "online",
            ServerStatus::Offline => "offline",
            ServerStatus::Unknown => "unknown",
        }
    }

    /// Parse a `ServerStatus` out of the string stored in its database column.
    ///
    /// Total and lossy by design: an unrecognised value yields
    /// [`ServerStatus::Unknown`] rather than an error, so a row written by a newer
    /// schema can never make an existing row unreadable.
    ///
    /// Deliberately not [`std::str::FromStr`]: that trait would need
    /// `Err = Infallible`, which hides the fallback from callers and makes
    /// `"nonsense".parse()` look like a successful parse.
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "online" => ServerStatus::Online,
            "offline" => ServerStatus::Offline,
            _ => ServerStatus::Unknown,
        }
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
