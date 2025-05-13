use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_seen: DateTime<Utc>,
}
