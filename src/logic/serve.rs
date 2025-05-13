use crate::logic::types::DeviceInfo;
use chrono::{DateTime, Utc};
use daemonize::Daemonize;
use std::{net::IpAddr, path::PathBuf, sync::Arc};
use warp::{http::StatusCode, Filter};

#[derive(serde::Deserialize)]
struct PingPayload {
    device_id: String,
}

pub async fn run(host: String, port: u16, db_path: String, daemon: bool) -> anyhow::Result<()> {
    if daemon {
        Daemonize::new()
            .pid_file("pulson.pid")
            .chown_pid_file(false)
            .working_directory(".")
            .start()?;
    }

    // expand ~ and env vars
    let db_path = shellexpand::tilde(&db_path).into_owned();
    let db = Arc::new(sled::open(PathBuf::from(db_path))?);

    // POST /ping → record device_id => now
    let ping_db = db.clone();
    let ping = warp::post()
        .and(warp::path("ping"))
        .and(warp::body::json())
        .map(move |payload: PingPayload| {
            let ts = Utc::now().to_rfc3339();
            let _ = ping_db.insert(payload.device_id.as_bytes(), ts.as_bytes());
            StatusCode::OK
        });

    // GET /devices → list all { device_id, last_seen }
    let list_db = db.clone();
    let list = warp::get().and(warp::path("devices")).map(move || {
        let mut devices = Vec::new();
        for item in list_db.iter().flatten() {
            if let (Ok(id), Ok(ts_str)) = (
                String::from_utf8(item.0.to_vec()),
                String::from_utf8(item.1.to_vec()),
            ) {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&ts_str) {
                    devices.push(DeviceInfo {
                        device_id: id,
                        last_seen: dt.with_timezone(&Utc),
                    });
                }
            }
        }
        warp::reply::json(&devices)
    });

    println!("pulson server running on http://{}:{}", host, port);
    let ip: IpAddr = host.parse()?;
    warp::serve(ping.or(list)).run((ip, port)).await;

    Ok(())
}
