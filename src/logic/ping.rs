use serde::Serialize;

#[derive(Serialize)]
struct PingPayload {
    device_id: String,
}

pub async fn run(host: String, port: u16, device_id: String) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!("http://{}:{}/ping", host, port);

    let resp = client
        .post(&url)
        .json(&PingPayload { device_id })
        .send()
        .await?;

    if resp.status().is_success() {
        println!("✓ Pinged {} successfully", url);
    } else {
        eprintln!("✗ Ping to {} failed: HTTP {}", url, resp.status());
    }

    Ok(())
}
