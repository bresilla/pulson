use crate::cli::{OutputFormat, SortBy, StatusFilter};
use crate::logic::config::StatusConfig;
use crate::logic::types::{DeviceInfo, TopicInfo, DeviceStatus, TopicStatus};
use chrono::Utc;
use reqwest::Client;
use serde_json;
use std::time::Duration;
use tokio::time::sleep;
use colored::*;

/// Get colored status indicator for device status
fn get_device_status_indicator(status: &DeviceStatus) -> String {
    match status {
        DeviceStatus::Online => "●".green().to_string(),
        DeviceStatus::Warning => "●".yellow().to_string(),
        DeviceStatus::Offline => "●".red().to_string(),
    }
}

/// Get colored status indicator for topic status
fn get_topic_status_indicator(status: &TopicStatus) -> String {
    match status {
        TopicStatus::Active => "●".green().to_string(),
        TopicStatus::Recent => "●".yellow().to_string(),
        TopicStatus::Stale => "●".bright_red().to_string(),
        TopicStatus::Inactive => "●".red().to_string(),
    }
}

/// Format age in seconds to human readable string
fn format_age(seconds: i64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}d", seconds / 86400)
    }
}

/// Sort devices based on the provided criteria
fn sort_devices(devices: &mut Vec<DeviceInfo>, sort_by: &SortBy, _config: &StatusConfig) {
    match sort_by {
        SortBy::LastSeen => devices.sort_by(|a, b| b.last_seen.cmp(&a.last_seen)),
        SortBy::Name => devices.sort_by(|a, b| a.device_id.cmp(&b.device_id)), // Only device_id available
        SortBy::Status => devices.sort_by(|a, b| {
            let a_priority = match a.status {
                DeviceStatus::Online => 0,
                DeviceStatus::Warning => 1,
                DeviceStatus::Offline => 2,
            };
            let b_priority = match b.status {
                DeviceStatus::Online => 0,
                DeviceStatus::Warning => 1,
                DeviceStatus::Offline => 2,
            };
            a_priority.cmp(&b_priority)
        }),
        _ => devices.sort_by(|a, b| b.last_seen.cmp(&a.last_seen)), // Default to last_seen
    }
}

/// Sort topics based on the provided criteria
fn sort_topics(topics: &mut Vec<TopicInfo>, sort_by: &SortBy, _config: &StatusConfig) {
    match sort_by {
        SortBy::LastSeen => topics.sort_by(|a, b| b.last_seen.cmp(&a.last_seen)),
        SortBy::Name => topics.sort_by(|a, b| a.topic.cmp(&b.topic)),
        SortBy::Status => topics.sort_by(|a, b| {
            let a_priority = match a.status {
                TopicStatus::Active => 0,
                TopicStatus::Recent => 1,
                TopicStatus::Stale => 2,
                TopicStatus::Inactive => 3,
            };
            let b_priority = match b.status {
                TopicStatus::Active => 0,
                TopicStatus::Recent => 1,
                TopicStatus::Stale => 2,
                TopicStatus::Inactive => 3,
            };
            a_priority.cmp(&b_priority)
        }),
        _ => topics.sort_by(|a, b| b.last_seen.cmp(&a.last_seen)),
    }
}

/// Filter devices by status
fn filter_devices(devices: Vec<DeviceInfo>, status_filter: &Option<StatusFilter>, _config: &StatusConfig) -> Vec<DeviceInfo> {
    if let Some(filter) = status_filter {
        devices.into_iter().filter(|device| {
            match filter {
                StatusFilter::Online => device.status == DeviceStatus::Online,
                StatusFilter::Warning => device.status == DeviceStatus::Warning,
                StatusFilter::Offline => device.status == DeviceStatus::Offline,
                _ => true, // Topic-specific filters don't apply to devices
            }
        }).collect()
    } else {
        devices
    }
}

/// Filter topics by status
fn filter_topics(topics: Vec<TopicInfo>, status_filter: &Option<StatusFilter>, _config: &StatusConfig) -> Vec<TopicInfo> {
    if let Some(filter) = status_filter {
        topics.into_iter().filter(|topic| {
            match filter {
                StatusFilter::Active => topic.status == TopicStatus::Active,
                StatusFilter::Recent => topic.status == TopicStatus::Recent,
                StatusFilter::Stale => topic.status == TopicStatus::Stale,
                StatusFilter::Inactive => topic.status == TopicStatus::Inactive,
                _ => true, // Device-specific filters don't apply to topics
            }
        }).collect()
    } else {
        topics
    }
}

/// Display devices in table format
fn display_devices_table(devices: &[DeviceInfo], extended: bool, _config: &StatusConfig) {
    if extended {
        println!("{}", "┌─────────────────────────────────────────────────────────────────────────────────────────────┐".bright_blue());
        println!("{}", "│                                         DEVICES                                                │".bright_blue().bold());
        println!("{}", "├─────────────────────────────────────────────────────────────────────────────────────────────┤".bright_blue());
        println!("{:<3} {:<25} {:<25} {:<10}", 
                 "ST".bright_white().bold(), 
                 "DEVICE ID".bright_white().bold(), 
                 "LAST SEEN".bright_white().bold(), 
                 "AGE".bright_white().bold());
        println!("{}", "├─────────────────────────────────────────────────────────────────────────────────────────────┤".bright_blue());
    } else {
        println!("{:<3} {:<25} {:<25} {:<10}", 
                 "ST".bright_white().bold(), 
                 "DEVICE ID".bright_white().bold(), 
                 "LAST SEEN".bright_white().bold(), 
                 "AGE".bright_white().bold());
        println!("{}", "─".repeat(65).bright_blue());
    }

    let now = Utc::now();
    for device in devices {
        let age_secs = now.signed_duration_since(device.last_seen).num_seconds();
        let status_indicator = get_device_status_indicator(&device.status);
        let device_id = if device.device_id.len() > 23 {
            format!("{}...", &device.device_id[..20])
        } else {
            device.device_id.clone()
        };

        println!("{:<3} {:<25} {:<25} {:<10}",
                 status_indicator,
                 device_id,
                 device.last_seen.format("%Y-%m-%d %H:%M:%S"),
                 format_age(age_secs));
    }

    if extended {
        println!("{}", "└─────────────────────────────────────────────────────────────────────────────────────────────┘".bright_blue());
    }
}

/// Display topics in table format
fn display_topics_table(topics: &[TopicInfo], device_id: &str, extended: bool, _config: &StatusConfig) {
    if extended {
        println!("{}", "┌─────────────────────────────────────────────────────────────────────────────────────────────┐".bright_green());
        println!("{}", format!("│                               TOPICS FOR {}                               │", device_id).bright_green().bold());
        println!("{}", "├─────────────────────────────────────────────────────────────────────────────────────────────┤".bright_green());
        println!("{:<3} {:<35} {:<25} {:<10}", 
                 "ST".bright_white().bold(), 
                 "TOPIC".bright_white().bold(), 
                 "LAST PING".bright_white().bold(), 
                 "AGE".bright_white().bold());
        println!("{}", "├─────────────────────────────────────────────────────────────────────────────────────────────┤".bright_green());
    } else {
        println!("\n{} {}", "Topics for:".bright_green().bold(), device_id.bright_white().bold());
        println!("{:<3} {:<35} {:<25} {:<10}", 
                 "ST".bright_white().bold(), 
                 "TOPIC".bright_white().bold(), 
                 "LAST PING".bright_white().bold(), 
                 "AGE".bright_white().bold());
        println!("{}", "─".repeat(75).bright_green());
    }

    let now = Utc::now();
    for topic in topics {
        let age_secs = now.signed_duration_since(topic.last_seen).num_seconds();
        let status_indicator = get_topic_status_indicator(&topic.status);
        let topic_name = if topic.topic.len() > 33 {
            format!("{}...", &topic.topic[..30])
        } else {
            topic.topic.clone()
        };

        println!("{:<3} {:<35} {:<25} {:<10}",
                 status_indicator,
                 topic_name,
                 topic.last_seen.format("%Y-%m-%d %H:%M:%S"),
                 format_age(age_secs));
    }

    if extended {
        println!("{}", "└─────────────────────────────────────────────────────────────────────────────────────────────┘".bright_green());
    }
}

/// Display devices in compact format
fn display_devices_compact(devices: &[DeviceInfo], _config: &StatusConfig) {
    println!("{} {}", "Devices:".bright_blue().bold(), devices.len());
    for device in devices {
        let status_indicator = get_device_status_indicator(&device.status);
        let age_secs = Utc::now().signed_duration_since(device.last_seen).num_seconds();
        println!("{} {} ({})", status_indicator, device.device_id, format_age(age_secs));
    }
}

/// Display topics in compact format
fn display_topics_compact(topics: &[TopicInfo], device_id: &str, _config: &StatusConfig) {
    println!("{} {} topics for {}", "Topics:".bright_green().bold(), topics.len(), device_id);
    for topic in topics {
        let status_indicator = get_topic_status_indicator(&topic.status);
        let age_secs = Utc::now().signed_duration_since(topic.last_seen).num_seconds();
        println!("{} {} ({})", status_indicator, topic.topic, format_age(age_secs));
    }
}

pub async fn run(
    host: String,
    port: u16,
    device_id: Option<String>,
    token: String,
    format: OutputFormat,
    sort: SortBy,
    status: Option<StatusFilter>,
    watch: bool,
    interval: u64,
    extended: bool,
    config: StatusConfig,
) -> anyhow::Result<()> {
    let client = Client::new();

    if watch {
        // Watch mode - continuously update
        println!("{}", "Watch mode enabled. Press Ctrl+C to exit...".bright_cyan().bold());
        loop {
            // Clear screen (ANSI escape code)
            print!("\x1B[2J\x1B[1;1H");
            
            // Print timestamp
            println!("{} {}", "Last updated:".bright_cyan(), Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
            println!();

            if let Err(e) = run_single_fetch(&client, &host, port, &device_id, &token, &format, &sort, &status, extended, &config).await {
                eprintln!("{} {}", "Error:".red().bold(), e);
            }

            sleep(Duration::from_secs(interval)).await;
        }
    } else {
        // Single fetch
        run_single_fetch(&client, &host, port, &device_id, &token, &format, &sort, &status, extended, &config).await
    }
}
async fn run_single_fetch(
    client: &Client,
    host: &str,
    port: u16,
    device_id: &Option<String>,
    token: &str,
    format: &OutputFormat,
    sort: &SortBy,
    status: &Option<StatusFilter>,
    extended: bool,
    config: &StatusConfig,
) -> anyhow::Result<()> {
    if let Some(dev) = device_id {
        // Fetch topics for specific device
        let url = format!("http://{}:{}/api/devices/{}", host, port, dev);
        let resp = client.get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "Server responded with status {}: {}",
                resp.status(),
                resp.text().await?
            );
        }

        // first deserialize into a serde_json::Value so we can branch on its shape
        let json_val: serde_json::Value = resp.json().await?;
        // now extract an array of TopicInfo from either an array or a map containing "topics"
        let mut topics: Vec<TopicInfo> = if let Some(arr) = json_val.as_array() {
            // raw [... ] at top level
            serde_json::from_value(serde_json::Value::Array(arr.clone()))?
        } else if let Some(arr) = json_val.get("topics").and_then(|v| v.as_array()) {
            // wrapped { "topics": [ ... ] }
            serde_json::from_value(serde_json::Value::Array(arr.clone()))?
        } else {
            anyhow::bail!("Unexpected response format: {}", json_val);
        };

        // Filter and sort topics
        topics = filter_topics(topics, status, config);
        sort_topics(&mut topics, sort, config);

        // Display topics
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&topics)?);
            }
            OutputFormat::Table => {
                if topics.is_empty() {
                    println!("{}", "No topics found for this device.".yellow());
                } else {
                    display_topics_table(&topics, dev, extended, config);
                }
            }
            OutputFormat::Compact => {
                if topics.is_empty() {
                    println!("{}", "No topics found for this device.".yellow());
                } else {
                    display_topics_compact(&topics, dev, config);
                }
            }
        }
    } else {
        // Fetch all devices
        let url = format!("http://{}:{}/api/devices", host, port);
        let resp = client.get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "Server responded with status {}: {}",
                resp.status(),
                resp.text().await?
            );
        }

        let mut devices: Vec<DeviceInfo> = resp.json().await?;
        
        // Filter and sort devices
        devices = filter_devices(devices, status, config);
        sort_devices(&mut devices, sort, config);

        // Display devices
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&devices)?);
            }
            OutputFormat::Table => {
                if devices.is_empty() {
                    println!("{}", "No devices found.".yellow());
                } else {
                    display_devices_table(&devices, extended, config);
                    
                    // Show summary
                    if extended {
                        let online = devices
                            .iter()
                            .filter(|d| d.status == DeviceStatus::Online)
                            .count();
                        let warning = devices
                            .iter()
                            .filter(|d| d.status == DeviceStatus::Warning)
                            .count();
                        let offline = devices
                            .iter()
                            .filter(|d| d.status == DeviceStatus::Offline)
                            .count();
                        
                        println!();
                        println!(
                            "{} {} online, {} warning, {} offline | {} total devices",
                            "Summary:".bright_white().bold(),
                            online.to_string().green(),
                            warning.to_string().yellow(),
                            offline.to_string().red(),
                            devices.len().to_string().bright_blue()
                        );
                    }
                }
            }
            OutputFormat::Compact => {
                if devices.is_empty() {
                    println!("{}", "No devices found.".yellow());
                } else {
                    display_devices_compact(&devices, config);
                }
            }
        }
    }
    Ok(())
}