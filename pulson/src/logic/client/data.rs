use reqwest::Client;
use serde::Serialize;

#[derive(Serialize)]
struct DataPayload {
    device_id: String,
    topic: String,
    data_type: String,
    data: serde_json::Value,
}

pub async fn run(
    host: String,
    port: u16,
    device_id: String,
    topic: String,
    data_type: String,
    data: String,
    token: String,
) -> anyhow::Result<()> {
    let client = Client::new();
    let url = format!("http://{}:{}/api/data", host, port);

    // Parse the JSON data
    let parsed_data: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| anyhow::anyhow!("Invalid JSON data: {}", e))?;

    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&DataPayload { 
            device_id, 
            topic, 
            data_type,
            data: parsed_data 
        })
        .send()
        .await?;

    if resp.status().is_success() {
        println!("✓ Data sent to {}", url);
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("✗ Data send failed: HTTP {} - {}", status, error_text);
    }

    Ok(())
}