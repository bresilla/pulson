use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::{path::PathBuf, sync::Arc};
use warp::{http::StatusCode, Filter};

#[derive(Parser)]
#[command(name = "pulson")]
/// realtime system/robot monitoring and tracing
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the HTTP server
    Serve {
        /// Address to bind (e.g. 0.0.0.0)
        #[arg(short, long, default_value = "0.0.0.0")]
        host: String,
        /// Port to listen on
        #[arg(short, long, default_value_t = 3030)]
        port: u16,
        /// Path to database file
        #[arg(short, long, default_value = ".db")]
        db_path: PathBuf,
        /// Run as daemon in background (Unix only)
        #[arg(long)]
        daemon: bool,
    },

    /// Query the running server for all tracked devices
    List {
        /// Address where pulson is running
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
        /// Port where pulson is listening
        #[arg(short, long, default_value_t = 3030)]
        port: u16,
    },
}

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
    let args = Cli::parse();

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

            // open sled database
            let db = Arc::new(sled::open(db_path)?);

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
                for item in list_db.iter() {
                    if let Ok((k, v)) = item {
                        if let (Ok(id), Ok(ts_str)) =
                            (String::from_utf8(k.to_vec()), String::from_utf8(v.to_vec()))
                        {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(&ts_str) {
                                devices.push(DeviceInfo {
                                    device_id: id,
                                    last_seen: dt.with_timezone(&Utc),
                                });
                            }
                        }
                    }
                }
                warp::reply::json(&devices)
            });

            let routes = ping.or(list);
            let ip: IpAddr = host.parse()?;
            println!("pulson server running on http://{}:{}", host, port);
            warp::serve(routes).run((ip, port)).await;
        }

        Commands::List { host, port } => {
            // HTTP GET /devices
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
