use crate::logic::serve::auth::authenticated_user;
use crate::logic::serve::database::{Database, get_device_data, list_user_devices, delete_device as db_delete_device, get_user_config_or_default, set_user_config as db_set_user_config, get_pulse_history, get_pulse_stats, store_device_data_payload, get_device_latest_data};
use crate::logic::config::StatusConfig;
use chrono::Utc;
use serde_json;
use std::sync::{Arc, Mutex};
use warp::{
    body::{json as warp_body_json, content_length_limit}, http::StatusCode, reply::{json as warp_json, with_status}, Filter, Rejection,
};

#[derive(serde::Deserialize)]
pub struct PulsePayload {
    pub device_id: String,
    pub topic: String,
    pub data: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
pub struct DeleteDevicePayload {
    pub device_id: String,
}

#[derive(serde::Deserialize)]
struct ConfigUpdateRequest {
    online_threshold_seconds: u64,
    warning_threshold_seconds: u64,
    stale_threshold_seconds: u64,
}

pub fn pulse(
    db: Database,
    save_images: bool,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path!("api" / "pulse"))
        .and(auth)
        .and(content_length_limit(50 * 1024 * 1024)) // 10MB limit for large images
        .and(warp_body_json())
        .map(move |username: String, payload: PulsePayload| {
            let ts = Utc::now().to_rfc3339();
            // Include username in device_id to isolate user data
            let device_id = format!("{}:{}", username, payload.device_id);
            
            match &payload.data {
                Some(data_value) => {
                    // Store data using new type system - it will automatically detect the type
                    match store_device_data_payload(&db, &device_id, Some(&payload.device_id), &payload.topic, "", data_value, &ts, save_images) {
                        Ok(_) => {
                            println!("Data pulse from device {} (user: {}) - topic: {}", 
                                payload.device_id, username, payload.topic);
                            with_status(
                                warp_json(&serde_json::json!({ "message": "pulse with data received" })),
                                StatusCode::OK,
                            )
                        }
                        Err(status_code) => {
                            eprintln!("Failed to store pulse data for device {} (user: {})", payload.device_id, username);
                            with_status(
                                warp_json(&serde_json::json!({ "error": "pulse data storage failed" })),
                                status_code,
                            )
                        }
                    }
                }
                None => {
                    // Handle simple ping - store as null data
                    let ping_data = serde_json::json!(null);
                    
                    match store_device_data_payload(&db, &device_id, Some(&payload.device_id), &payload.topic, "", &ping_data, &ts, save_images) {
                        Ok(_) => {
                            println!("Ping pulse from device {} (user: {})", payload.device_id, username);
                            with_status(
                                warp_json(&serde_json::json!({ "message": "ping pulse received" })),
                                StatusCode::OK,
                            )
                        }
                        Err(status_code) => {
                            eprintln!("Failed to store ping pulse for device {} (user: {})", payload.device_id, username);
                            with_status(
                                warp_json(&serde_json::json!({ "error": "ping pulse failed" })),
                                status_code,
                            )
                        }
                    }
                }
            }
        })
}

pub fn list_all(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "devices"))
        .and(warp::path::end())
        .and(auth)
        .map(move |username: String| {
            // Get user's personal configuration
            let config = get_user_config_or_default(&db, &username);
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
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "devices" / String))
        .and(auth)
        .map(move |device_id: String, username: String| {
            // Include username in device_id to get user-specific device
            let full_device_id = format!("{}:{}", username, device_id);
            let config = get_user_config_or_default(&db, &username);
            
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

// reload_config route removed - no longer needed with purely server-based configuration

pub fn get_config(
    status_config: Arc<Mutex<StatusConfig>>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path!("api" / "config"))
        .and(warp::path::end())
        .map(move || {
            let config = status_config.lock().unwrap().clone();
            with_status(
                warp_json(&serde_json::json!({
                    "online_threshold_seconds": config.online_threshold_seconds,
                    "warning_threshold_seconds": config.warning_threshold_seconds,
                    "stale_threshold_seconds": config.stale_threshold_seconds
                })),
                StatusCode::OK,
            )
        })
}

pub fn update_config(
    status_config: Arc<Mutex<StatusConfig>>,
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db);
    warp::post()
        .and(warp::path!("api" / "config" / "update"))
        .and(warp::path::end())
        .and(auth)
        .and(warp_body_json())
        .map(move |_username: String, payload: ConfigUpdateRequest| {
            // Validate thresholds
            if payload.online_threshold_seconds >= payload.warning_threshold_seconds {
                return with_status(
                    warp_json(&serde_json::json!({ 
                        "error": "Online threshold must be less than warning threshold" 
                    })),
                    StatusCode::BAD_REQUEST,
                );
            }
            
            if payload.warning_threshold_seconds >= payload.stale_threshold_seconds {
                return with_status(
                    warp_json(&serde_json::json!({ 
                        "error": "Warning threshold must be less than stale threshold" 
                    })),
                    StatusCode::BAD_REQUEST,
                );
            }

            // Update in-memory configuration
            {
                let mut config = status_config.lock().unwrap();
                config.online_threshold_seconds = payload.online_threshold_seconds;
                config.warning_threshold_seconds = payload.warning_threshold_seconds;
                config.stale_threshold_seconds = payload.stale_threshold_seconds;
            }

            with_status(
                warp_json(&serde_json::json!({ 
                    "message": "Configuration updated successfully" 
                })),
                StatusCode::OK,
            )
        })
}

/// GET /api/user/config - Get user's personal configuration
pub fn get_user_config(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "user" / "config"))
        .and(warp::path::end())
        .and(auth)
        .map(move |username: String| {
            let config = get_user_config_or_default(&db, &username);
            with_status(
                warp_json(&serde_json::json!({
                    "online_threshold_seconds": config.online_threshold_seconds,
                    "warning_threshold_seconds": config.warning_threshold_seconds,
                    "stale_threshold_seconds": config.stale_threshold_seconds
                })),
                StatusCode::OK,
            )
        })
}

/// POST /api/user/config - Set user's personal configuration
pub fn set_user_config(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path!("api" / "user" / "config"))
        .and(warp::path::end())
        .and(auth)
        .and(warp_body_json())
        .map(move |username: String, payload: ConfigUpdateRequest| {
            // Validate thresholds
            if payload.online_threshold_seconds >= payload.warning_threshold_seconds {
                return with_status(
                    warp_json(&serde_json::json!({ 
                        "error": "Online threshold must be less than warning threshold" 
                    })),
                    StatusCode::BAD_REQUEST,
                );
            }
            
            if payload.warning_threshold_seconds >= payload.stale_threshold_seconds {
                return with_status(
                    warp_json(&serde_json::json!({ 
                        "error": "Warning threshold must be less than stale threshold" 
                    })),
                    StatusCode::BAD_REQUEST,
                );
            }

            let config = StatusConfig {
                online_threshold_seconds: payload.online_threshold_seconds,
                warning_threshold_seconds: payload.warning_threshold_seconds,
                stale_threshold_seconds: payload.stale_threshold_seconds,
            };

            match db_set_user_config(&db, &username, &config) {
                Ok(_) => {
                    with_status(
                        warp_json(&serde_json::json!({ 
                            "message": "User configuration updated successfully" 
                        })),
                        StatusCode::OK,
                    )
                }
                Err(_) => {
                    with_status(
                        warp_json(&serde_json::json!({ 
                            "error": "Failed to update user configuration" 
                        })),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            }
        })
}

/// GET /api/devices/{device_id}/history?time_range={1h|1d|1w|1m}&topic={topic_name} - Get pulse history for visualization
pub fn get_device_history(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "devices" / String / "history"))
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(auth)
        .map(move |device_id: String, params: std::collections::HashMap<String, String>, username: String| {
            // Include username in device_id to get user-specific device
            let full_device_id = format!("{}:{}", username, device_id);
            
            let time_range = params.get("time_range").map(|s| s.as_str()).unwrap_or("1d");
            let topic = params.get("topic").map(|s| s.as_str());
            
            match get_pulse_history(&db, &full_device_id, topic, time_range) {
                Ok(history_data) => {
                    with_status(warp_json(&history_data), StatusCode::OK)
                }
                Err(status_code) => {
                    eprintln!("Failed to get pulse history for device {} (user: {})", device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ 
                            "error": "Failed to get pulse history" 
                        })),
                        status_code,
                    )
                }
            }
        })
}

/// GET /api/devices/{device_id}/stats?time_range={1h|1d|1w|1m} - Get pulse statistics
pub fn get_device_stats(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "devices" / String / "stats"))
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(auth)
        .map(move |device_id: String, params: std::collections::HashMap<String, String>, username: String| {
            // Include username in device_id to get user-specific device
            let full_device_id = format!("{}:{}", username, device_id);
            
            let time_range = params.get("time_range").map(|s| s.as_str()).unwrap_or("1d");
            
            match get_pulse_stats(&db, &full_device_id, time_range) {
                Ok(stats_data) => {
                    with_status(warp_json(&stats_data), StatusCode::OK)
                }
                Err(status_code) => {
                    eprintln!("Failed to get pulse stats for device {} (user: {})", device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ 
                            "error": "Failed to get pulse statistics" 
                        })),
                        status_code,
                    )
                }
            }
        })
}

/// GET /api/devices/{device_id}/data?topic={topic_name}&type={data_type} - Get latest data for a device
pub fn get_device_data_latest(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "devices" / String / "data"))
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(auth)
        .map(move |device_id: String, params: std::collections::HashMap<String, String>, username: String| {
            // Include username in device_id to get user-specific device
            let full_device_id = format!("{}:{}", username, device_id);
            
            let topic = params.get("topic").map(|s| s.as_str());
            let data_type = params.get("type").map(|s| s.as_str());
            
            match get_device_latest_data(&db, &full_device_id, topic, data_type) {
                Ok(data_response) => {
                    with_status(warp_json(&data_response), StatusCode::OK)
                }
                Err(status_code) => {
                    eprintln!("Failed to get latest data for device {} (user: {})", device_id, username);
                    with_status(
                        warp_json(&serde_json::json!({ 
                            "error": "Failed to get latest data" 
                        })),
                        status_code,
                    )
                }
            }
        })
}
