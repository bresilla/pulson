use reqwest::Client;
use serde::Serialize;

#[derive(Serialize)]
struct PingPayload {
    device_id: String,
    topic: String,
}

pub async fn run(
    host: String,
    port: u16,
    device_id: String,
    topic: String,
    token: String,
) -> anyhow::Result<()> {
    let client = Client::new();
    let url = format!("http://{}:{}/ping", host, port);

    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&PingPayload { device_id, topic })
        .send()
        .await?;

    if resp.status().is_success() {
        println!("✓ Pinged {}", url);
    } else {
        eprintln!("✗ Ping failed: HTTP {}", resp.status());
    }

    Ok(())
}
