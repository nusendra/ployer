use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub id: String,
    pub application_id: String,
    pub domain: String,
    pub is_primary: bool,
    pub ssl_active: bool,
    /// Serve as a wildcard (*.<domain>) plus the apex, for tenant subdomains.
    #[serde(default)]
    pub wildcard: bool,
    pub created_at: DateTime<Utc>,
}
