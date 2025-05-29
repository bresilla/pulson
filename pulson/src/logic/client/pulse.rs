use reqwest::Client;
use serde::Serialize;
use crate::cli::DataType;
use crate::logic::client::url_utils::build_api_url;
use serde_json::json;
use image::io::Reader as ImageReader;

#[derive(Serialize)]
struct PulsePayload {
    device_id: String,
    topic: String,
    data: Option<serde_json::Value>,
}

pub async fn run(
    base_url: Option<String>,
    host: String,
    port: u16,
    device_id: String,
    topic: String,
    data_type: DataType,
    data: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<f64>,
    value: Option<f64>,
    state: Option<bool>,
    message: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    image_file: Option<String>,
    token: String,
) -> anyhow::Result<()> {
    let client = Client::new();
    let url = build_api_url(base_url.as_deref(), &host, port, "/api/pulse");

    // Generate appropriate JSON data based on data type and parameters
    let json_data = if let Some(ref custom_data) = data {
        // If custom JSON data is provided, use it directly
        Some(serde_json::from_str(custom_data)
            .map_err(|e| anyhow::anyhow!("Invalid JSON data: {}", e))?)
    } else {
        // Generate data based on data type and provided parameters
        match data_type {
            DataType::Pulse => {
                // Simple pulse with no data payload
                None
            },
            DataType::Gps => {
                if let (Some(lat), Some(lon)) = (latitude, longitude) {
                    Some(json!({
                        "GPS": {
                            "lat": lat,
                            "lon": lon,
                            "alt": altitude
                        }
                    }))
                } else {
                    return Err(anyhow::anyhow!("GPS data type requires --latitude and --longitude parameters"));
                }
            },
            DataType::Sensor => {
                if let Some(sensor_value) = value {
                    Some(json!({
                        "sensor": sensor_value
                    }))
                } else {
                    return Err(anyhow::anyhow!("Sensor data type requires --value parameter"));
                }
            },
            DataType::Trigger => {
                if let Some(trigger_state) = state {
                    Some(json!({
                        "trigger": trigger_state
                    }))
                } else {
                    return Err(anyhow::anyhow!("Trigger data type requires --state parameter"));
                }
            },
            DataType::Event => {
                if let Some(ref event_message) = message {
                    Some(json!({
                        "event": event_message
                    }))
                } else {
                    return Err(anyhow::anyhow!("Event data type requires --message parameter"));
                }
            },
            DataType::Image => {
                if let Some(ref file_path) = image_file {
                    // Read and decode image file to RGB pixels
                    let img = ImageReader::open(file_path)
                        .map_err(|e| anyhow::anyhow!("Failed to open image file '{}': {}", file_path, e))?
                        .decode()
                        .map_err(|e| anyhow::anyhow!("Failed to decode image file '{}': {}", file_path, e))?;
                    
                    // Convert to RGB format
                    let rgb_img = img.to_rgb8();
                    let (img_width, img_height) = rgb_img.dimensions();
                    let image_data = rgb_img.into_raw();
                    let channels = 3; // RGB
                    
                    println!("📷 Loaded image: {}x{} RGB ({} bytes)", img_width, img_height, image_data.len());
                    
                    Some(json!({
                        "image": {
                            "rows": img_height,
                            "cols": img_width,
                            "channels": channels,
                            "data": image_data
                        }
                    }))
                } else if let (Some(img_width), Some(img_height)) = (width, height) {
                    // Generate dummy image data for demonstration
                    let channels = 3; // RGB
                    let data_size = (img_width * img_height * channels) as usize;
                    let dummy_data: Vec<u8> = (0..data_size).map(|i| (i % 256) as u8).collect();
                    
                    Some(json!({
                        "image": {
                            "rows": img_height,
                            "cols": img_width,
                            "channels": channels,
                            "data": dummy_data
                        }
                    }))
                } else {
                    return Err(anyhow::anyhow!("Image data type requires either --image-file or both --width and --height parameters"));
                }
            },
        }
    };

    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&PulsePayload { 
            device_id: device_id.clone(), 
            topic: topic.clone(), 
            data: json_data 
        })
        .send()
        .await?;

    if resp.status().is_success() {
        if data.is_some() {
            println!("✓ Custom data sent to {}", url);
        } else {
            match data_type {
                DataType::Pulse => println!("✓ Pulse sent to {}", url),
                DataType::Gps => println!("✓ GPS data sent to {} (lat: {:.6}, lon: {:.6})", url, latitude.unwrap(), longitude.unwrap()),
                DataType::Sensor => println!("✓ Sensor data sent to {} (value: {})", url, value.unwrap()),
                DataType::Trigger => println!("✓ Trigger data sent to {} (state: {})", url, state.unwrap()),
                DataType::Event => println!("✓ Event data sent to {} (message: '{}')", url, message.as_ref().unwrap()),
                DataType::Image => println!("✓ Image data sent to {} ({}x{} pixels)", url, width.unwrap(), height.unwrap()),
            }
        }
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("✗ Pulse failed: HTTP {} - {}", status, error_text);
    }

    Ok(())
}
