use chrono::{DateTime, Utc};
use daemonize::Daemonize;
use serde_json::json;
use std::{collections::HashMap, net::IpAddr, path::PathBuf, sync::Arc};
use uuid::Uuid;
use warp::reply::{json as warp_json, with_status};
use warp::{http::StatusCode, Filter, Rejection};

use crate::logic::types::{DeviceInfo, TopicInfo};

/// Custom rejection for auth failures
#[derive(Debug)]
struct Unauthorized;
impl warp::reject::Reject for Unauthorized {}

#[derive(serde::Deserialize)]
struct AccountPayload {
    username: String,
    password: String,
}

#[derive(serde::Deserialize)]
struct PingPayload {
    device_id: String,
    topic: String,
}

pub async fn run(host: String, port: u16, db_path: String, daemon: bool) -> anyhow::Result<()> {
    // 1) Daemonize if requested
    if daemon {
        Daemonize::new()
            .pid_file("pulson.pid")
            .chown_pid_file(false)
            .working_directory(".")
            .start()?;
    }

    // 2) Open sled DB (expanding ~)
    let expanded = shellexpand::tilde(&db_path).into_owned();
    let db = Arc::new(sled::open(PathBuf::from(expanded))?);

    //
    // 3) Unprotected endpoints: register & login
    //

    // POST /account/register
    let reg_db = db.clone();
    let register = warp::post()
        .and(warp::path("account"))
        .and(warp::path("register"))
        .and(warp::body::json())
        .map(move |payload: AccountPayload| {
            let key = format!("user:{}", payload.username);
            if reg_db.contains_key(key.as_bytes()).unwrap_or(false) {
                StatusCode::CONFLICT
            } else {
                let _ = reg_db.insert(key.as_bytes(), payload.password.as_bytes());
                StatusCode::CREATED
            }
        });

    // POST /account/login
    let login_db = db.clone();
    let login = warp::post()
        .and(warp::path("account"))
        .and(warp::path("login"))
        .and(warp::body::json())
        .map(move |payload: AccountPayload| {
            let key = format!("user:{}", payload.username);
            let err_reply = || {
                with_status(
                    warp_json(&json!({ "error": "invalid credentials" })),
                    StatusCode::UNAUTHORIZED,
                )
            };

            match login_db.get(key.as_bytes()).ok().flatten() {
                Some(stored) if stored == payload.password.as_bytes() => {
                    let token = Uuid::new_v4().to_string();
                    let _ = login_db.insert(
                        format!("token:{}", token).as_bytes(),
                        payload.username.as_bytes(),
                    );
                    with_status(warp_json(&json!({ "token": token })), StatusCode::OK)
                }
                _ => err_reply(),
            }
        });

    //
    // 4) Auth filter that yields `username: String`
    //

    let auth_user = {
        let db = db.clone();
        warp::header::optional::<String>("authorization").and_then(move |auth: Option<String>| {
            let db = db.clone();
            async move {
                let header = auth.ok_or_else(|| warp::reject::custom(Unauthorized))?;
                let token = header
                    .strip_prefix("Bearer ")
                    .ok_or_else(|| warp::reject::custom(Unauthorized))?;
                let key = format!("token:{}", token);
                match db.get(key.as_bytes()).ok().flatten() {
                    Some(user_bytes) => {
                        let user = String::from_utf8(user_bytes.to_vec())
                            .map_err(|_| warp::reject::custom(Unauthorized))?;
                        Ok(user)
                    }
                    _ => Err(warp::reject::custom(Unauthorized)),
                }
            }
        })
    };

    //
    // 5) Protected endpoints: ping + list
    //

    // POST /ping
    let ping_db = db.clone();
    let ping = warp::post()
        .and(warp::path("ping"))
        .and(auth_user.clone())
        .and(warp::body::json())
        .map(move |username: String, payload: PingPayload| {
            let ts = Utc::now().to_rfc3339();
            let key = format!("{}|{}|{}", username, payload.device_id, payload.topic);
            let _ = ping_db.insert(key.as_bytes(), ts.as_bytes());
            StatusCode::OK
        });

    // GET /devices → list all devices for this user
    let list_all = {
        let db = db.clone();
        warp::get()
            .and(warp::path("devices"))
            .and(warp::path::end())
            .and(auth_user.clone())
            .map(move |username: String| {
                let mut latest: HashMap<String, DateTime<Utc>> = HashMap::new();
                for item in db.iter().flatten() {
                    if let (Ok(k), Ok(v)) = (
                        String::from_utf8(item.0.to_vec()),
                        String::from_utf8(item.1.to_vec()),
                    ) {
                        // key = "{user}|{device}|{topic}"
                        if let Some(rest) = k.strip_prefix(&format!("{}|", username)) {
                            // rest = "{device}|{topic}"
                            if let Some((device, _)) = rest.split_once('|') {
                                if let Ok(dt) = DateTime::parse_from_rfc3339(&v) {
                                    let ts = dt.with_timezone(&Utc);
                                    latest
                                        .entry(device.to_string())
                                        .and_modify(|old| {
                                            if ts > *old {
                                                *old = ts
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
    };

    // GET /devices/{device_id} → list topics for that device, for this user
    let list_one = {
        let db = db.clone();
        warp::get()
            .and(warp::path("devices"))
            .and(warp::path::param::<String>())
            .and(auth_user.clone())
            .map(move |device_id: String, username: String| {
                let mut topics = Vec::new();
                for item in db.iter().flatten() {
                    if let (Ok(k), Ok(v)) = (
                        String::from_utf8(item.0.to_vec()),
                        String::from_utf8(item.1.to_vec()),
                    ) {
                        // key = "{user}|{device}|{topic}"
                        let prefix = format!("{}|{}|", username, device_id);
                        if let Some(topic) = k.strip_prefix(&prefix) {
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
    };

    //
    // 6) Combine & recover Unauthorized → 401 JSON error
    //

    let routes = register
        .or(login)
        .or(ping)
        .or(list_one)
        .or(list_all)
        .recover(|err: Rejection| async move {
            if err.find::<Unauthorized>().is_some() {
                Ok(with_status(
                    warp_json(&json!({ "error": "Unauthorized" })),
                    StatusCode::UNAUTHORIZED,
                ))
            } else {
                Err(err)
            }
        });

    println!("pulson server running on http://{}:{}", host, port);
    let ip: IpAddr = host.parse()?;
    warp::serve(routes).run((ip, port)).await;

    Ok(())
}
