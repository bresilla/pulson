use crate::logic::types::DeviceInfo;
use chrono::Utc;

pub async fn run(host: String, port: u16) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/devices", host, port);
    let devices: Vec<DeviceInfo> = reqwest::get(&url).await?.json().await?;

    println!(
        "{:<20} {:<25} {:<10}",
        "DEVICE ID", "LAST SEEN (UTC)", "AGE"
    );
    let now = Utc::now();
    for d in devices {
        let age_s = now.signed_duration_since(d.last_seen).num_seconds();
        println!("{:<20} {:<25} {}s", d.device_id, d.last_seen, age_s);
    }

    Ok(())
}
