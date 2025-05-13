use crate::logic::types::{DeviceInfo, TopicInfo};
use chrono::Utc;

/// Turn a raw seconds‐count into “<n>s”, “<n>m”, “<n>h” or “<n>d”
fn format_age(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

pub async fn run(host: String, port: u16, device_id: Option<String>) -> anyhow::Result<()> {
    let now = Utc::now();

    if let Some(dev) = device_id {
        // per‐device view: topics + humanized age
        let url = format!("http://{}:{}/devices/{}", host, port, dev);
        let topics: Vec<TopicInfo> = reqwest::get(&url).await?.json().await?;

        println!("{:<30} {:<25} {:<10}", "TOPIC", "LAST SEEN (UTC)", "AGE");
        for t in topics {
            let secs = now.signed_duration_since(t.last_seen).num_seconds();
            println!(
                "{:<30} {:<25} {:<10}",
                t.topic,
                t.last_seen,
                format_age(secs)
            );
        }
    } else {
        // global view: devices + humanized age
        let url = format!("http://{}:{}/devices", host, port);
        let devices: Vec<DeviceInfo> = reqwest::get(&url).await?.json().await?;

        println!(
            "{:<20} {:<25} {:<10}",
            "DEVICE ID", "LAST SEEN (UTC)", "AGE"
        );
        for d in devices {
            let secs = now.signed_duration_since(d.last_seen).num_seconds();
            println!(
                "{:<20} {:<25} {:<10}",
                d.device_id,
                d.last_seen,
                format_age(secs)
            );
        }
    }

    Ok(())
}
