use crate::logic::serve::auth::authenticated_user;
use crate::logic::serve::database::{Database, store_device_data, get_device_data, list_user_devices, delete_device as db_delete_device};
use crate::logic::config::StatusConfig;
use chrono::Utc;
use serde_json;
use std::sync::{Arc, Mutex};
use warp::{
    body::json as warp_body_json, http::StatusCode, reply::{json as warp_json, with_status}, Filter, Rejection,
};

#[derive(serde::Deserialize)]
pub struct PingPayload {
    pub device_id: String,
    pub topic: String,
}

#[derive(serde::Deserialize)]
pub struct DeleteDevicePayload {
    pub device_id: String,
}

pub fn ping(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path!("api" / "ping"))
        .and(auth)
        .and(warp_body_json())
        .map(move |username: String, payload: PingPayload| {
            let ts = Utc::now().to_rfc3339();
            // Include username in device_id to isolate user data
            let device_id = format!("{}:{}", username, payload.device_id);
            let data = serde_json::json!({
                "topic": payload.topic,
                "timestamp": ts.clone()
            }).to_string();
            
            match store_device_data(&db, &device_id, Some(&payload.device_id), &data, &ts) {
                Ok(_) => {
                    println!("Ping from device {} (user: {})", payload.device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ "message": "ping received" })),
                        StatusCode::OK,
                    )
                }
                Err(status_code) => {
                    eprintln!("Failed to store ping for device {} (user: {})", payload.device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ "error": "ping failed" })),
                        status_code,
                    )
                }
            }
        })
}

pub fn list_all(
    db: Database,
    status_config: Arc<Mutex<StatusConfig>>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "devices"))
        .and(warp::path::end())
        .and(auth)
        .map(move |username: String| {
            // Get devices for the authenticated user
            let config = status_config.lock().unwrap().clone();
            match list_user_devices(&db, &username, &config) {
                Ok(devices_json) => warp_json(&devices_json),
                Err(_) => {
                    eprintln!("Failed to list devices for user: {}", username);
                    warp_json(&serde_json::json!({"error": "failed to list devices"}))
                }
            }
        })
}

pub fn list_one(
    db: Database,
    status_config: Arc<Mutex<StatusConfig>>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "devices" / String))
        .and(auth)
        .map(move |device_id: String, username: String| {
            // Include username in device_id to get user-specific device
            let full_device_id = format!("{}:{}", username, device_id);
            let config = status_config.lock().unwrap().clone();
            
            match get_device_data(&db, &full_device_id, &config) {
                Ok(Some(topics_json)) => {
                    // Parse the topics JSON and return it directly
                    if let Ok(topics) = serde_json::from_str::<serde_json::Value>(&topics_json) {
                        warp_json(&topics)
                    } else {
                        warp_json(&serde_json::json!([]))
                    }
                }
                Ok(None) => {
                    warp_json(&serde_json::json!([]))
                }
                Err(_) => {
                    eprintln!("Failed to get device {} for user: {}", device_id, username);
                    warp_json(&serde_json::json!({
                        "error": "failed to get device data"
                    }))
                }
            }
        })
}

pub fn delete_device(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path!("api" / "device" / "delete"))
        .and(warp::path::end())
        .and(auth)
        .and(warp_body_json())
        .map(move |username: String, payload: DeleteDevicePayload| {
            // Include username in device_id for user-specific deletion
            let full_device_id = format!("{}:{}", username, payload.device_id);
            
            match db_delete_device(&db, &full_device_id) {
                Ok(true) => {
                    println!("Deleted device {} (user: {})", payload.device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ "message": "device deleted" })),
                        StatusCode::OK,
                    )
                }
                Ok(false) => {
                    println!("Device {} not found for deletion (user: {})", payload.device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ "error": "device not found" })),
                        StatusCode::NOT_FOUND,
                    )
                }
                Err(status_code) => {
                    eprintln!("Failed to delete device {} (user: {})", payload.device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ "error": "failed to delete device" })),
                        status_code,
                    )
                }
            }
        })
}

pub fn reload_config(
    status_config: Arc<Mutex<StatusConfig>>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("api" / "config" / "reload"))
        .and(warp::path::end())
        .map(move || {
            // Get default configuration path
            let config_path = {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.config/pulson/config.toml", home)
            };
            
            match StatusConfig::from_file(&config_path) {
                Ok(new_config) => {
                    let mut config = status_config.lock().unwrap();
                    *config = new_config;
                    println!("Configuration reloaded successfully from: {}", config_path);
                    with_status(
                        warp_json(&serde_json::json!({ "message": "configuration reloaded" })),
                        StatusCode::OK,
                    )
                }
                Err(e) => {
                    eprintln!("Failed to reload configuration from {}: {}", config_path, e);
                    with_status(
                        warp_json(&serde_json::json!({ "error": "failed to reload configuration" })),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            }
        })
}
