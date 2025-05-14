// pulson/src/logic/serve.rs

use chrono::{DateTime, Utc};
use daemonize::Daemonize;
use pulson_ui::ui_routes;
use serde_json::json;
use std::{net::IpAddr, path::PathBuf, sync::Arc};
use uuid::Uuid;
use warp::reply::{json as warp_json, with_status};
use warp::{http::StatusCode, Filter, Rejection};

use crate::logic::types::{DeviceInfo, TopicInfo};

/// Rejection type for auth failures
#[derive(Debug)]
struct Unauthorized;
impl warp::reject::Reject for Unauthorized {}

/// Payloads
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
    _webui: bool, // still accepted, but unused at routing level
) -> anyhow::Result<()> {
    // 1) Daemonize if requested
    if daemon {
        Daemonize::new()
            .pid_file("pulson.pid")
            .chown_pid_file(false)
            .working_directory(".")
            .start()?;
    }

    // 2) Open Sled (expanding ~)
    let expanded = shellexpand::tilde(&db_path).into_owned();
    let db = Arc::new(sled::open(PathBuf::from(expanded))?);

    //
    // 3) ACCOUNT /register
    //
    let reg_db = db.clone();
    let register = warp::post()
        .and(warp::path!("account" / "register"))
        .and(warp::body::json())
        .map(move |payload: AccountPayload| {
            let key = format!("user:{}", payload.username);
            if reg_db.contains_key(key.as_bytes()).unwrap_or(false) {
                return StatusCode::CONFLICT;
            }
            let _ = reg_db.insert(key.as_bytes(), payload.password.as_bytes());

            let role = if payload
                .rootpass
                .as_ref()
                .and_then(|rp| root_pass.as_ref().map(|rp2| rp == rp2))
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

    //
    // 4) ACCOUNT /login
    //
    let login_db = db.clone();
    let login = warp::post()
        .and(warp::path!("account" / "login"))
        .and(warp::body::json())
        .map(move |payload: AccountPayload| {
            let key = format!("user:{}", payload.username);
            let err = || {
                with_status(
                    warp_json(&json!({ "error": "invalid credentials" })),
                    StatusCode::UNAUTHORIZED,
                )
            };
            match login_db
                .get(key.as_bytes())
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

    //
    // 5) Authentication filter
    //
    let auth = authenticated_user(db.clone());

    //
    // 6) ACCOUNT /delete/<user>
    //
    let del_db = db.clone();
    let delete_user = warp::delete()
        .and(warp::path!("account" / String))
        .and(auth.clone())
        .map(move |target: String, caller: String| {
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
    // 7) ACCOUNT /users  (list all users, root only)
    //
    let list_users = {
        let db = db.clone();
        warp::get()
            .and(warp::path!("account" / "users"))
            .and(auth.clone())
            .map(move |caller: String| {
                let role_key = format!("role:{}", caller);
                let is_root = db
                    .get(role_key.as_bytes())
                    .ok()
                    .flatten()
                    .map(|v| v.as_ref() == b"root")
                    .unwrap_or(false);
                if !is_root {
                    return with_status(
                        warp_json(&json!({ "error": "forbidden" })),
                        StatusCode::FORBIDDEN,
                    );
                }

                let mut users = Vec::new();
                for item in db.iter().flatten() {
                    let key = String::from_utf8_lossy(&item.0);
                    if let Some(name) = key.strip_prefix("user:") {
                        let role = db
                            .get(format!("role:{}", name).as_bytes())
                            .ok()
                            .flatten()
                            .and_then(|v| String::from_utf8(v.to_vec()).ok())
                            .unwrap_or_else(|| "user".into());
                        users.push(json!({ "username": name, "role": role }));
                    }
                }

                with_status(warp_json(&users), StatusCode::OK)
            })
    };

    //
    // 8) POST /ping
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
    // 9) GET /devices
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
    };

    //
    // 10) GET /devices/<device_id>
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
    // 11) Combine all API routes and handle Unauthorized
    //
    let api = register
        .or(login)
        .or(delete_user)
        .or(list_users)
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
        })
        .boxed();

    //
    // 12) ALWAYS mount the WebUI static routes
    //
    let routes = api.or(ui_routes()).boxed();

    println!("pulson server running on http://{}:{}", host, port);
    let ip: IpAddr = host.parse()?;
    warp::serve(routes).run((ip, port)).await;

    Ok(())
}

/// Filter that checks Authorization header and returns username
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
                Some(bytes) => {
                    let user = String::from_utf8(bytes.to_vec())
                        .map_err(|_| warp::reject::custom(Unauthorized))?;
                    Ok(user)
                }
                None => Err(warp::reject::custom(Unauthorized)),
            }
        }
    })
}
