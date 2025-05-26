use crate::logic::serve::auth::authenticated_user;
use crate::logic::types::{DeviceInfo, TopicInfo};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use warp::{
    body::json as warp_body_json, http::StatusCode, reply::json as warp_json, Filter, Rejection,
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
    db: Arc<sled::Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path("ping"))
        .and(auth)
        .and(warp_body_json())
        .map(move |username: String, payload: PingPayload| {
            let ts = Utc::now().to_rfc3339();
            // CRITICAL FIX: Include username in key to isolate user data
            let key = format!("{}:{}|{}", username, payload.device_id, payload.topic);
            let _ = db.insert(key.as_bytes(), ts.as_bytes());
            StatusCode::OK
        })
}

pub fn list_all(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path("devices"))
        .and(warp::path::end())
        .and(auth)
        .map(move |username: String| {
            let mut latest = std::collections::HashMap::new();
            let user_prefix = format!("{}:", username);
            
            for item in db.iter().flatten() {
                if let (Ok(k), Ok(v)) = (
                    String::from_utf8(item.0.to_vec()),
                    String::from_utf8(item.1.to_vec()),
                ) {
                    // CRITICAL FIX: Only process keys that belong to this user
                    if k.starts_with(&user_prefix) {
                        // Remove the "username:" prefix and then split on "|"
                        let key_without_user = &k[user_prefix.len()..];
                        if let Some((dev, _)) = key_without_user.split_once('|') {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(&v) {
                                let ts = dt.with_timezone(&Utc);
                                latest
                                    .entry(dev.to_string())
                                    .and_modify(|old: &mut DateTime<Utc>| {
                                        if ts > *old {
                                            *old = ts;
                                        }
                                    })
                                    .or_insert(ts);
                            }
                        }
                    }
                }
            }
            let devices: Vec<DeviceInfo> = latest
                .into_iter()
                .map(|(device_id, last_seen)| DeviceInfo {
                    device_id,
                    last_seen,
                })
                .collect();
            warp_json(&devices)
        })
}

pub fn list_one(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path("devices"))
        .and(warp::path::param::<String>())
        .and(auth)
        .map(move |device_id: String, username: String| {
            let mut topics = Vec::new();
            let user_prefix = format!("{}:", username);
            let device_prefix = format!("{}{}|", user_prefix, device_id);
            
            for item in db.iter().flatten() {
                if let (Ok(k), Ok(v)) = (
                    String::from_utf8(item.0.to_vec()),
                    String::from_utf8(item.1.to_vec()),
                ) {
                    // CRITICAL FIX: Only process keys that belong to this user and device
                    if k.starts_with(&device_prefix) {
                        // Extract the topic part after "username:device|"
                        let topic = &k[device_prefix.len()..];
                        if let Ok(dt) = DateTime::parse_from_rfc3339(&v) {
                            topics.push(TopicInfo {
                                topic: topic.to_string(),
                                last_seen: dt.with_timezone(&Utc),
                            });
                        }
                    }
                }
            }
            warp_json(&topics)
        })
}

pub fn delete_device(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path("device"))
        .and(warp::path("delete"))
        .and(warp::path::end())
        .and(auth)
        .and(warp_body_json())
        .map(move |username: String, payload: DeleteDevicePayload| {
            let user_prefix = format!("{}:", username);
            let device_prefix_to_delete = format!("{}{}|", user_prefix, payload.device_id);
            let mut deleted_count = 0;

            // Iterate over all keys and remove those matching the pattern
            // This is not the most efficient way for large datasets,
            // but sled does not directly support prefix deletion in a single command.
            for item in db.iter() {
                match item {
                    Ok((key_bytes, _)) => {
                        if let Ok(key_str) = String::from_utf8(key_bytes.to_vec()) {
                            if key_str.starts_with(&device_prefix_to_delete) {
                                match db.remove(&key_bytes) {
                                    Ok(_) => deleted_count += 1,
                                    Err(e) => {
                                        eprintln!("Failed to delete key {}: {}", key_str, e);
                                        // Optionally, decide if this should be a hard error
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error iterating db for deletion: {}", e);
                        // Optionally, decide if this should be a hard error
                    }
                }
            }

            if deleted_count > 0 {
                // Successfully deleted some entries
                StatusCode::OK
            } else {
                // No entries found for that device_id for that user, or deletion failed silently for all
                // Consider if a 404 Not Found might be more appropriate if no keys were found
                StatusCode::NOT_FOUND // Or OK if "nothing to delete" is also a success
            }
        })
}
