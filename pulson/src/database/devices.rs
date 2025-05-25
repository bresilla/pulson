use crate::database::Database;
use anyhow::Result;
use chrono::{DateTime, Utc};
use limbo::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: i64,
    pub device_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub status: DeviceStatus,
    pub topic_count: i64,
    pub ping_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: i64,
    pub device_id: i64,
    pub topic: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
    pub ping_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicInfo {
    pub topic: String,
    pub description: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub status: TopicStatus,
    pub ping_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceStatus {
    Online,   // Active within last 30 seconds
    Warning,  // Active within last 5 minutes
    Offline,  // No activity beyond 5 minutes
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TopicStatus {
    Active,   // Pinged within last 30 seconds
    Recent,   // Pinged within last 5 minutes
    Stale,    // Pinged within last hour
    Inactive, // No pings beyond 1 hour
}

impl DeviceStatus {
    pub fn from_last_seen(last_seen: DateTime<Utc>) -> Self {
        let now = Utc::now();
        let diff = now.signed_duration_since(last_seen);
        
        if diff.num_seconds() < 30 {
            DeviceStatus::Online
        } else if diff.num_minutes() < 5 {
            DeviceStatus::Warning
        } else {
            DeviceStatus::Offline
        }
    }
}

impl TopicStatus {
    pub fn from_last_ping(last_ping: DateTime<Utc>) -> Self {
        let now = Utc::now();
        let diff = now.signed_duration_since(last_ping);
        
        if diff.num_seconds() < 30 {
            TopicStatus::Active
        } else if diff.num_minutes() < 5 {
            TopicStatus::Recent
        } else if diff.num_hours() < 1 {
            TopicStatus::Stale
        } else {
            TopicStatus::Inactive
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PingRequest {
    pub device_id: String,
    pub topic: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Database {
    /// Get or create a device, updating last_seen time
    pub async fn upsert_device(&self, device_id: &str) -> Result<i64> {
        // First check if device exists
        if let Some(device) = self.get_device_by_id(device_id).await? {
            // Update last_seen
            self.execute(
                "UPDATE devices SET last_seen = CURRENT_TIMESTAMP, is_active = 1 WHERE device_id = ?1",
                params!(device_id),
            ).await?;
            Ok(device.id)
        } else {
            // Create new device
            self.execute(
                "INSERT INTO devices (device_id, created_at, updated_at, last_seen) 
                 VALUES (?1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                params!(device_id),
            ).await?;
            
            // Get the new device ID
            if let Some(device) = self.get_device_by_id(device_id).await? {
                Ok(device.id)
            } else {
                Err(anyhow::anyhow!("Failed to create device"))
            }
        }
    }

    /// Get device by device_id
    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Option<Device>> {
        self.query_row(
            "SELECT id, device_id, name, description, created_at, updated_at, last_seen, is_active 
             FROM devices WHERE device_id = ?1 AND is_active = 1",
            params!(device_id),
            |row| {
                Ok(Device {
                    id: row.get::<i64>(0)?,
                    device_id: row.get::<String>(1)?,
                    name: row.get::<Option<String>>(2)?,
                    description: row.get::<Option<String>>(3)?,
                    created_at: row.get::<DateTime<Utc>>(4)?,
                    updated_at: row.get::<DateTime<Utc>>(5)?,
                    last_seen: row.get::<DateTime<Utc>>(6)?,
                    is_active: row.get::<bool>(7)?,
                })
            },
        ).await
    }

    /// List all devices with statistics
    pub async fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        self.query_rows(
            "SELECT 
                d.device_id, 
                d.name, 
                d.description, 
                d.last_seen,
                COUNT(DISTINCT t.id) as topic_count,
                COALESCE(SUM(t.ping_count), 0) as ping_count
             FROM devices d
             LEFT JOIN topics t ON d.id = t.device_id
             WHERE d.is_active = 1
             GROUP BY d.id, d.device_id, d.name, d.description, d.last_seen
             ORDER BY d.last_seen DESC",
            params!(),
            |row| {
                let last_seen: DateTime<Utc> = row.get::<DateTime<Utc>>(3)?;
                Ok(DeviceInfo {
                    device_id: row.get::<String>(0)?,
                    name: row.get::<Option<String>>(1)?,
                    description: row.get::<Option<String>>(2)?,
                    last_seen,
                    status: DeviceStatus::from_last_seen(last_seen),
                    topic_count: row.get::<i64>(4)?,
                    ping_count: row.get::<i64>(5)?,
                })
            },
        ).await
    }

    /// Update device information
    pub async fn update_device(&self, device_id: &str, name: Option<&str>, description: Option<&str>) -> Result<bool> {
        self.execute(
            "UPDATE devices SET name = ?1, description = ?2 
             WHERE device_id = ?3 AND is_active = 1",
            params!(name, description, device_id),
        ).await?;
        
        Ok(true)
    }

    /// Get or create a topic for a device
    pub async fn upsert_topic(&self, device_db_id: i64, topic: &str) -> Result<i64> {
        // First check if topic exists
        if let Some(existing_topic) = self.query_row(
            "SELECT id FROM topics WHERE device_id = ?1 AND topic = ?2",
            params!(device_db_id, topic),
            |row| Ok(row.get::<i64>(0)?),
        ).await? {
            // Update ping count and last_ping
            self.execute(
                "UPDATE topics SET last_ping = CURRENT_TIMESTAMP, ping_count = ping_count + 1 
                 WHERE id = ?1",
                params!(existing_topic),
            ).await?;
            Ok(existing_topic)
        } else {
            // Create new topic
            self.execute(
                "INSERT INTO topics (device_id, topic, created_at, updated_at, last_ping, ping_count) 
                 VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 1)",
                params!(device_db_id, topic),
            ).await?;
            
            // Get the new topic ID
            if let Some(topic_id) = self.query_row(
                "SELECT id FROM topics WHERE device_id = ?1 AND topic = ?2",
                params!(device_db_id, topic),
                |row| Ok(row.get::<i64>(0)?),
            ).await? {
                Ok(topic_id)
            } else {
                Err(anyhow::anyhow!("Failed to create topic"))
            }
        }
    }

    /// List topics for a specific device
    pub async fn list_topics_for_device(&self, device_id: &str) -> Result<Vec<TopicInfo>> {
        self.query_rows(
            "SELECT t.topic, t.description, t.last_ping, t.ping_count
             FROM topics t
             JOIN devices d ON t.device_id = d.id
             WHERE d.device_id = ?1 AND d.is_active = 1
             ORDER BY t.last_ping DESC",
            params!(device_id),
            |row| {
                let last_ping: DateTime<Utc> = row.get::<DateTime<Utc>>(2)?;
                Ok(TopicInfo {
                    topic: row.get::<String>(0)?,
                    description: row.get::<Option<String>>(1)?,
                    last_seen: last_ping,
                    status: TopicStatus::from_last_ping(last_ping),
                    ping_count: row.get::<i64>(3)?,
                })
            },
        ).await
    }

    /// Record a ping for a device and topic
    pub async fn record_ping(&self, device_id: &str, topic: &str, user_id: i64, metadata: Option<&str>) -> Result<()> {
        // Get or create device
        let device_db_id = self.upsert_device(device_id).await?;
        
        // Get or create topic
        let topic_db_id = self.upsert_topic(device_db_id, topic).await?;
        
        // Record the ping
        self.execute(
            "INSERT INTO pings (device_id, topic_id, user_id, timestamp, metadata) 
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP, ?4)",
            params!(device_db_id, topic_db_id, user_id, metadata),
        ).await?;
        
        Ok(())
    }

    /// Get device statistics
    pub async fn get_device_stats(&self) -> Result<serde_json::Value> {
        let total_devices = self.query_row(
            "SELECT COUNT(*) FROM devices WHERE is_active = 1",
            params!(),
            |row| Ok(row.get::<i64>(0)?),
        ).await?.unwrap_or(0);
        
        let online_devices = self.query_row(
            "SELECT COUNT(*) FROM devices 
             WHERE is_active = 1 AND last_seen > datetime('now', '-30 seconds')",
            params!(),
            |row| Ok(row.get::<i64>(0)?),
        ).await?.unwrap_or(0);
        
        let total_topics = self.query_row(
            "SELECT COUNT(*) FROM topics",
            params!(),
            |row| Ok(row.get::<i64>(0)?),
        ).await?.unwrap_or(0);
        
        let pings_24h = self.query_row(
            "SELECT COUNT(*) FROM pings WHERE timestamp > datetime('now', '-1 day')",
            params!(),
            |row| Ok(row.get::<i64>(0)?),
        ).await?.unwrap_or(0);
        
        Ok(serde_json::json!({
            "total_devices": total_devices,
            "online_devices": online_devices,
            "total_topics": total_topics,
            "pings_24h": pings_24h
        }))
    }

    /// Get recent pings for a device
    pub async fn get_recent_pings(&self, device_id: &str, limit: i64) -> Result<Vec<serde_json::Value>> {
        self.query_rows(
            "SELECT t.topic, p.timestamp, p.metadata, u.username
             FROM pings p
             JOIN topics t ON p.topic_id = t.id
             JOIN devices d ON p.device_id = d.id
             JOIN users u ON p.user_id = u.id
             WHERE d.device_id = ?1
             ORDER BY p.timestamp DESC
             LIMIT ?2",
            params!(device_id, limit),
            |row| {
                Ok(serde_json::json!({
                    "topic": row.get::<String>(0)?,
                    "timestamp": row.get::<DateTime<Utc>>(1)?,
                    "metadata": row.get::<Option<String>>(2)?,
                    "user": row.get::<String>(3)?
                }))
            },
        ).await
    }

    /// Deactivate a device
    pub async fn deactivate_device(&self, device_id: &str) -> Result<bool> {
        self.execute(
            "UPDATE devices SET is_active = 0 WHERE device_id = ?1",
            params!(device_id),
        ).await?;
        
        Ok(true)
    }

    /// Clean up old ping data
    pub async fn cleanup_old_pings(&self, days: i64) -> Result<()> {
        self.execute(
            "DELETE FROM pings WHERE timestamp < datetime('now', '-' || ?1 || ' days')",
            params!(days),
        ).await?;
        
        Ok(())
    }
}