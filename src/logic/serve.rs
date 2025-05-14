use chrono::{DateTime, Utc};
use daemonize::Daemonize;
use serde_json::json;
use std::{net::IpAddr, path::PathBuf, sync::Arc};
use uuid::Uuid;
use warp::reply::{json as warp_json, with_status};
use warp::{http::StatusCode, Filter, Rejection};

use crate::logic::types::{DeviceInfo, TopicInfo};

#[derive(Debug)]
struct Unauthorized;
impl warp::reject::Reject for Unauthorized {}

#[derive(serde::Deserialize)]
struct AccountPayload {
    username: String,
    password: String,
    rootpass: Option<String>,
}

#[derive(serde::Deserialize)]
struct PingPayload {
    device_id: String,
    topic: String,
}

pub async fn run(
    host: String,
    port: u16,
    db_path: String,
    daemon: bool,
    root_pass: Option<String>,
) -> anyhow::Result<()> {
    if daemon {
        Daemonize::new()
            .pid_file("pulson.pid")
            .chown_pid_file(false)
            .working_directory(".")
            .start()?;
    }

    let db = Arc::new(sled::open(PathBuf::from(
        shellexpand::tilde(&db_path).into_owned(),
    ))?);

    //
    // ACCOUNT: register / login / delete
    //

    // REGISTER
    let reg_db = db.clone();
    let register = warp::post()
        .and(warp::path!("account" / "register"))
        .and(warp::body::json())
        .map(move |payload: AccountPayload| {
            let user_key = format!("user:{}", payload.username);
            if reg_db.contains_key(user_key.as_bytes()).unwrap_or(false) {
                return StatusCode::CONFLICT;
            }
            // store password
            let _ = reg_db.insert(user_key.as_bytes(), payload.password.as_bytes());
            // figure out role
            let role = if payload
                .rootpass
                .as_ref()
                .and_then(|rp| root_pass.as_ref().map(|rp_s| rp == rp_s))
                .unwrap_or(false)
            {
                "root"
            } else {
                "user"
            };
            let role_key = format!("role:{}", payload.username);
            let _ = reg_db.insert(role_key.as_bytes(), role.as_bytes());
            StatusCode::CREATED
        });

    // LOGIN
    let login_db = db.clone();
    let login = warp::post()
        .and(warp::path!("account" / "login"))
        .and(warp::body::json())
        .map(move |payload: AccountPayload| {
            let user_key = format!("user:{}", payload.username);
            let err = || {
                with_status(
                    warp_json(&json!({ "error": "invalid credentials" })),
                    StatusCode::UNAUTHORIZED,
                )
            };
            match login_db
                .get(user_key.as_bytes())
                .ok()
                .flatten()
                .map(|v| v.to_vec())
            {
                Some(stored) if stored == payload.password.as_bytes() => {
                    let token = Uuid::new_v4().to_string();
                    let tok_key = format!("token:{}", token);
                    let _ = login_db.insert(tok_key.as_bytes(), payload.username.as_bytes());
                    with_status(warp_json(&json!({ "token": token })), StatusCode::OK)
                }
                _ => err(),
            }
        });

    // DELETE USER (root only)
    let del_db = db.clone();
    let delete_user = warp::delete()
        .and(warp::path!("account" / String))
        .and(authenticated_user(db.clone()))
        .map(move |target: String, caller: String| {
            // only root can delete
            let role_key = format!("role:{}", caller);
            if let Ok(Some(role)) = del_db.get(role_key.as_bytes()) {
                if role.as_ref() == b"root" {
                    let _ = del_db.remove(format!("user:{}", target).as_bytes());
                    let _ = del_db.remove(format!("role:{}", target).as_bytes());
                    return StatusCode::OK;
                }
            }
            StatusCode::FORBIDDEN
        });

    //
    // AUTH filter (yields username)
    //

    let auth = authenticated_user(db.clone());

    //
    // PING
    //

    let ping_db = db.clone();
    let ping = warp::post()
        .and(warp::path("ping"))
        .and(auth.clone())
        .and(warp::body::json())
        .map(move |_: String, payload: PingPayload| {
            let ts = Utc::now().to_rfc3339();
            let key = format!("{}|{}", payload.device_id, payload.topic);
            let _ = ping_db.insert(key.as_bytes(), ts.as_bytes());
            StatusCode::OK
        });

    //
    // LIST ALL DEVICES
    //

    let list_all = {
        let db = db.clone();
        warp::get()
            .and(warp::path("devices"))
            .and(warp::path::end())
            .and(auth.clone())
            .map(move |_: String| {
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
                warp_json(&devices)
            })
    };

    //
    // LIST ONE DEVICE’S TOPICS
    //

    let list_one = {
        let db = db.clone();
        warp::get()
            .and(warp::path("devices"))
            .and(warp::path::param::<String>())
            .and(auth.clone())
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
    };

    //
    // COMBINE & RECOVER
    //

    let routes = register
        .or(login)
        .or(delete_user)
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

// filter returning the authenticated username
fn authenticated_user(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization").and_then(move |auth: Option<String>| {
        let db = db.clone();
        async move {
            let header = auth.ok_or_else(|| warp::reject::custom(Unauthorized))?;
            let token = header
                .strip_prefix("Bearer ")
                .ok_or_else(|| warp::reject::custom(Unauthorized))?;
            let key = format!("token:{}", token);
            match db.get(key.as_bytes()).ok().flatten() {
                Some(user) => Ok(String::from_utf8(user.to_vec()).unwrap()),
                _ => Err(warp::reject::custom(Unauthorized)),
            }
        }
    })
}
