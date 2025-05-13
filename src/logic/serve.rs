use chrono::{DateTime, Utc};
use std::{net::IpAddr, path::PathBuf, sync::Arc};
use warp::{http::StatusCode, Filter};

use crate::logic::types::{DeviceInfo, TopicInfo};

#[derive(serde::Deserialize)]
struct PingPayload {
    device_id: String,
    topic: String,
}

pub async fn run(host: String, port: u16, db_path: String, daemon: bool) -> anyhow::Result<()> {
    if daemon {
        daemonize::Daemonize::new()
            .pid_file("pulson.pid")
            .chown_pid_file(false)
            .working_directory(".")
            .start()?;
    }

    let db_path = shellexpand::tilde(&db_path).into_owned();
    let db = Arc::new(sled::open(PathBuf::from(db_path))?);

    // POST /ping → record (device,topic) → now
    let ping_db = db.clone();
    let ping = warp::post()
        .and(warp::path("ping"))
        .and(warp::body::json())
        .map(move |payload: PingPayload| {
            let ts = Utc::now().to_rfc3339();
            let key = format!("{}|{}", payload.device_id, payload.topic);
            let _ = ping_db.insert(key.as_bytes(), ts.as_bytes());
            StatusCode::OK
        });

    // GET /devices → list all unique devices
    let list_all = {
        let db = db.clone();
        warp::get()
            .and(warp::path("devices"))
            .and(warp::path::end())
            .map(move || {
                let mut latest = std::collections::HashMap::new();
                for item in db.iter().flatten() {
                    if let (Ok(k), Ok(v)) = (
                        String::from_utf8(item.0.to_vec()),
                        String::from_utf8(item.1.to_vec()),
                    ) {
                        if let Some((device, _)) = k.split_once('|') {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(&v) {
                                let ts = dt.with_timezone(&Utc);
                                latest
                                    .entry(device.to_string())
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
                warp::reply::json(&devices)
            })
    };

    // GET /devices/{device_id} → list topics for that device
    let list_one = {
        let db = db.clone();
        warp::get()
            .and(warp::path("devices"))
            .and(warp::path::param::<String>())
            .map(move |device_id: String| {
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
                warp::reply::json(&topics)
            })
    };

    let routes = ping.or(list_one).or(list_all);

    println!("pulson server running on http://{}:{}", host, port);
    let ip: IpAddr = host.parse()?;
    warp::serve(routes).run((ip, port)).await;

    Ok(())
}
