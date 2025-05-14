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

pub fn ping(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path("ping"))
        .and(auth)
        .and(warp_body_json())
        .map(move |_: String, payload: PingPayload| {
            let ts = Utc::now().to_rfc3339();
            let key = format!("{}|{}", payload.device_id, payload.topic);
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
        .map(move |_: String| {
            let mut latest = std::collections::HashMap::new();
            for item in db.iter().flatten() {
                if let (Ok(k), Ok(v)) = (
                    String::from_utf8(item.0.to_vec()),
                    String::from_utf8(item.1.to_vec()),
                ) {
                    if let Some((dev, _)) = k.split_once('|') {
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
        .map(move |device_id: String, _: String| {
            let mut topics = Vec::new();
            for item in db.iter().flatten() {
                if let (Ok(k), Ok(v)) = (
                    String::from_utf8(item.0.to_vec()),
                    String::from_utf8(item.1.to_vec()),
                ) {
                    if let Some((dev, topic)) = k.split_once('|') {
                        if dev == device_id {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(&v) {
                                topics.push(TopicInfo {
                                    topic: topic.to_string(),
                                    last_seen: dt.with_timezone(&Utc),
                                });
                            }
                        }
                    }
                }
            }
            warp_json(&topics)
        })
}
