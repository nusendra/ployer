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

    pub fn from_str(s: &str) -> Self {
        match s {
            "online" => ServerStatus::Online,
            "offline" => ServerStatus::Offline,
            _ => ServerStatus::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_status_round_trips() {
        for (variant, s) in [
            (ServerStatus::Online, "online"),
            (ServerStatus::Offline, "offline"),
            (ServerStatus::Unknown, "unknown"),
        ] {
            assert_eq!(variant.as_str(), s);
            assert_eq!(ServerStatus::from_str(s), variant);
        }
    }

    #[test]
    fn server_status_unknown_defaults_to_unknown() {
        assert_eq!(ServerStatus::from_str("unreachable"), ServerStatus::Unknown);
    }
}
