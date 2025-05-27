use reqwest::Client;
use serde::Serialize;

#[derive(Serialize)]
struct PulsePayload {
    device_id: String,
    topic: String,
    data: Option<serde_json::Value>,
}

pub async fn run(
    host: String,
    port: u16,
    device_id: String,
    topic: String,
    data: Option<String>,
    token: String,
) -> anyhow::Result<()> {
    let client = Client::new();
    let url = format!("http://{}:{}/api/pulse", host, port);

    // Parse the JSON data if provided
    let is_data_provided = data.is_some();
    let parsed_data = if let Some(data_str) = data {
        Some(serde_json::from_str(&data_str)
            .map_err(|e| anyhow::anyhow!("Invalid JSON data: {}", e))?)
    } else {
        // For simple ping, send empty ping data  
        Some(serde_json::json!({"ping": null}))
    };

    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&PulsePayload { 
            device_id: device_id.clone(), 
            topic: topic.clone(), 
            data: parsed_data 
        })
        .send()
        .await?;

    if resp.status().is_success() {
        if is_data_provided {
            println!("✓ Pulse with data sent to {}", url);
        } else {
            println!("✓ Ping pulse sent to {}", url);
        }
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("✗ Pulse failed: HTTP {} - {}", status, error_text);
    }

    Ok(())
}
