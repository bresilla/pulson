use serde::{Deserialize, Serialize};
use std::path::Path;
use chrono::{DateTime, Utc};
use crate::logic::types::{DeviceStatus, TopicStatus};

/// Configuration for device and topic status timing thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusConfig {
    /// Threshold for online/active status in seconds (default: 30)
    pub online_threshold_seconds: u64,
    /// Threshold for warning/recent status in seconds (default: 300)
    pub warning_threshold_seconds: u64,
    /// Threshold for stale status in seconds (only for topics, default: 3600)
    pub stale_threshold_seconds: u64,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            online_threshold_seconds: 30,
            warning_threshold_seconds: 300,
            stale_threshold_seconds: 3600,
        }
    }
}

impl StatusConfig {
    /// Load configuration from a TOML file, falling back to defaults if file doesn't exist
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: StatusConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Create configuration from command line arguments and environment variables
    pub fn from_args_and_env(
        online_threshold: Option<u64>,
        warning_threshold: Option<u64>,
        stale_threshold: Option<u64>,
    ) -> Self {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(val) = std::env::var("PULSON_ONLINE_THRESHOLD") {
            if let Ok(parsed) = val.parse() {
                config.online_threshold_seconds = parsed;
            }
        }
        if let Ok(val) = std::env::var("PULSON_WARNING_THRESHOLD") {
            if let Ok(parsed) = val.parse() {
                config.warning_threshold_seconds = parsed;
            }
        }
        if let Ok(val) = std::env::var("PULSON_STALE_THRESHOLD") {
            if let Ok(parsed) = val.parse() {
                config.stale_threshold_seconds = parsed;
            }
        }

        // Override with command line arguments (highest priority)
        if let Some(val) = online_threshold {
            config.online_threshold_seconds = val;
        }
        if let Some(val) = warning_threshold {
            config.warning_threshold_seconds = val;
        }
        if let Some(val) = stale_threshold {
            config.stale_threshold_seconds = val;
        }

        config
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Create a default configuration file at the given path if it doesn't exist
    pub fn ensure_default_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let config = Self::default();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            config.save_to_file(path)?;
            println!("Created default configuration file at: {}", path.display());
            Ok(config)
        } else {
            Self::from_file(path)
        }
    }

    /// Calculate device status based on last seen timestamp
    pub fn calculate_device_status(&self, last_seen: &DateTime<Utc>) -> DeviceStatus {
        let now = Utc::now();
        let diff = now.signed_duration_since(*last_seen);
        let seconds = diff.num_seconds() as u64;
        
        if seconds < self.online_threshold_seconds {
            DeviceStatus::Online
        } else if seconds < self.warning_threshold_seconds {
            DeviceStatus::Warning
        } else {
            DeviceStatus::Offline
        }
    }

    /// Calculate topic status based on last seen timestamp
    pub fn calculate_topic_status(&self, last_seen: &DateTime<Utc>) -> TopicStatus {
        let now = Utc::now();
        let diff = now.signed_duration_since(*last_seen);
        let seconds = diff.num_seconds() as u64;
        
        if seconds < self.online_threshold_seconds {
            TopicStatus::Active
        } else if seconds < self.warning_threshold_seconds {
            TopicStatus::Recent
        } else if seconds < self.stale_threshold_seconds {
            TopicStatus::Stale
        } else {
            TopicStatus::Inactive
        }
    }
}
