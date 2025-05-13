use crate::logic::types::DeviceInfo;
use chrono::Utc;
use reqwest::Client;

/// GET /devices, filter by age > threshold, then POST /devices/{id}/inactive
pub async fn run(host: String, port: u16, threshold: u64) -> anyhow::Result<()> {
    let base = format!("http://{}:{}", host, port);
    let client = Client::new();
    let now = Utc::now();

    // 1) fetch all devices
    let url = format!("{}/devices", base);
    let devices: Vec<DeviceInfo> = client.get(&url).send().await?.json().await?;

    // 2) filter stale
    let stale: Vec<_> = devices
        .into_iter()
        .filter(|d| now.signed_duration_since(d.last_seen).num_seconds() > threshold as i64)
        .collect();

    // 3) mark each inactive
    for d in stale {
        let url = format!("{}/devices/{}/inactive", base, d.device_id);
        let resp = client.post(&url).send().await?;
        if resp.status().is_success() {
            println!("✓ Marked {} inactive", d.device_id);
        } else {
            eprintln!("✗ Failed to mark {}: HTTP {}", d.device_id, resp.status());
        }
    }

    Ok(())
}
