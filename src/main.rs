mod cli;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf, sync::Arc};
use warp::{http::StatusCode, Filter};

use clap::Parser;
use cli::{Cli, Commands};

#[derive(Deserialize)]
struct PingPayload {
    device_id: String,
}

#[derive(Serialize, Deserialize)]
struct DeviceInfo {
    device_id: String,
    last_seen: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::try_parse()?;

    match args.command {
        Commands::Serve {
            host,
            port,
            db_path,
            daemon,
        } => {
            if daemon {
                use daemonize::Daemonize;
                Daemonize::new()
                    .pid_file("pulson.pid")
                    .chown_pid_file(false)
                    .working_directory(".")
                    .start()?;
            }

            // expand ~
            let db_path = shellexpand::tilde(&db_path).into_owned();
            let db = Arc::new(sled::open(PathBuf::from(db_path))?);

            // POST /ping
            let ping_db = db.clone();
            let ping = warp::post()
                .and(warp::path("ping"))
                .and(warp::body::json())
                .map(move |payload: PingPayload| {
                    let ts = Utc::now().to_rfc3339();
                    let _ = ping_db.insert(payload.device_id.as_bytes(), ts.as_bytes());
                    StatusCode::OK
                });

            // GET /devices
            let list_db = db.clone();
            let list = warp::get().and(warp::path("devices")).map(move || {
                let mut devices = Vec::new();
                for item in list_db.iter().flatten() {
                    if let (Ok(id), Ok(ts)) = (
                        String::from_utf8(item.0.to_vec()),
                        String::from_utf8(item.1.to_vec()),
                    ) {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(&ts) {
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
        }

        Commands::List { host, port } => {
            let url = format!("http://{}:{}/devices", host, port);
            let devices: Vec<DeviceInfo> = reqwest::get(&url).await?.json().await?;

            println!(
                "{:<20} {:<25} {:<10}",
                "DEVICE ID", "LAST SEEN (UTC)", "AGE"
            );
            let now = Utc::now();
            for d in devices {
                let age_s = now.signed_duration_since(d.last_seen).num_seconds();
                println!("{:<20} {:<25} {}s", d.device_id, d.last_seen, age_s);
            }
        }
    }

    Ok(())
}
