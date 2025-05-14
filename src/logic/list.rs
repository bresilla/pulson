use crate::logic::types::{DeviceInfo, TopicInfo};
use chrono::Utc;
use reqwest::Client;

/// Format age as s/m/h/d
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

pub async fn run(
    host: String,
    port: u16,
    device_id: Option<String>,
    token: String,
) -> anyhow::Result<()> {
    let now = Utc::now();
    let client = Client::new();

    if let Some(dev) = device_id {
        let url = format!("http://{}:{}/devices/{}", host, port, dev);
        let resp = client.get(&url).bearer_auth(&token).send().await?;

        if !resp.status().is_success() {
            eprintln!("Error: {}", resp.text().await?);
            return Ok(());
        }

        let topics: Vec<TopicInfo> = resp.json().await?;
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
        let url = format!("http://{}:{}/devices", host, port);
        let resp = client.get(&url).bearer_auth(&token).send().await?;

        if !resp.status().is_success() {
            eprintln!("Error: {}", resp.text().await?);
            return Ok(());
        }

        let devices: Vec<DeviceInfo> = resp.json().await?;
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
