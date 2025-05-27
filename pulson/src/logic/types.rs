use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device status enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Online,   // Active within online threshold
    Warning,  // Active within warning threshold  
    Offline,  // No activity beyond warning threshold
}

/// Topic status enum  
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopicStatus {
    Active,   // Pinged within online threshold
    Recent,   // Pinged within warning threshold
    Stale,    // Pinged within stale threshold
    Inactive, // No pings beyond stale threshold
}

/// Device ↔ last_seen summary with server-calculated status
#[derive(Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_seen: DateTime<Utc>,
    pub status: DeviceStatus,
}

/// Topic ↔ last_seen summary with server-calculated status
#[derive(Serialize, Deserialize)]
pub struct TopicInfo {
    pub topic: String,
    pub last_seen: DateTime<Utc>,
    pub status: TopicStatus,
    pub data_types: Vec<String>, // List of data types available for this topic
}
