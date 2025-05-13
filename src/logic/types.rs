use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device ↔ last_seen summary
#[derive(Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_seen: DateTime<Utc>,
}

/// Topic ↔ last_seen summary
#[derive(Serialize, Deserialize)]
pub struct TopicInfo {
    pub topic: String,
    pub last_seen: DateTime<Utc>,
}
