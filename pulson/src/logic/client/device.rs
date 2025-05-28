use reqwest::Client;
use serde::Serialize;
use crate::logic::client::url_utils::build_api_url;

#[derive(Serialize)]
struct DeleteDeviceRequest {
    device_id: String,
}

pub async fn delete(
    base_url: Option<String>,
    host: String,
    port: u16,
    device_id: String,
    token: String,
) -> anyhow::Result<()> {
    let client = Client::new();
    let url = build_api_url(base_url.as_deref(), &host, port, "/device/delete");

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&DeleteDeviceRequest { device_id: device_id.clone() })
        .send()
        .await?;

    if response.status().is_success() {
        println!("✓ Device '{}' deleted successfully.", device_id);
    } else {
        eprintln!("✗ Failed to delete device '{}': {}", device_id, response.status());
    }

    Ok(())
}
