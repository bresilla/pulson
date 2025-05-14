# Project Structure

```
./
└── pulson/
    ├── src/
    │   ├── logic/
    │   │   ├── client/
    │   │   │   ├── account.rs
    │   │   │   ├── list.rs
    │   │   │   ├── mod.rs
    │   │   │   └── ping.rs
    │   │   ├── serve/
    │   │   │   ├── api/
    │   │   │   │   ├── account_routes.rs
    │   │   │   │   ├── device_routes.rs
    │   │   │   │   └── mod.rs
    │   │   │   ├── auth.rs
    │   │   │   ├── mod.rs
    │   │   │   └── ui.rs
    │   │   ├── mod.rs
    │   │   └── types.rs
    │   ├── cli.rs
    │   └── main.rs
    ├── Cargo.toml
    └── build.rs
```

# Project Files

## File: `pulson/src/logic/client/account.rs`

```rust
use directories::ProjectDirs;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::{fs, io};

#[derive(Serialize)]
struct AccountPayload<'a> {
    username: &'a str,
    password: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    rootpass: Option<&'a str>,
}

fn token_file() -> io::Result<std::path::PathBuf> {
    let pd = ProjectDirs::from("com", "example", "pulson")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no config dir"))?;
    let dir = pd.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("token"))
}

pub fn read_token() -> io::Result<String> {
    let p = token_file()?;
    fs::read_to_string(p).map(|s| s.trim().to_string())
}

pub async fn register(
    host: String,
    port: u16,
    username: String,
    password: String,
    rootpass: Option<String>,
) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/account/register", host, port);
    let payload = AccountPayload {
        username: &username,
        password: &password,
        rootpass: rootpass.as_deref(),
    };
    let resp = Client::new().post(&url).json(&payload).send().await?;
    if resp.status().is_success() {
        println!("✓ Registered `{}`", username);
    } else {
        eprintln!("✗ Registration failed: {}", resp.text().await?);
    }
    Ok(())
}

pub async fn login(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/account/login", host, port);
    let payload = AccountPayload {
        username: &username,
        password: &password,
        rootpass: None,
    };
    let resp = Client::new().post(&url).json(&payload).send().await?;

    if resp.status().is_success() {
        // specify Value so .json() knows what to parse
        let json: Value = resp.json::<Value>().await?;
        let tok = json["token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("no token in response"))?
            .to_string();
        fs::write(token_file()?, &tok)?;
        println!("✓ Logged in");
    } else {
        eprintln!("✗ Login failed: {}", resp.text().await?);
    }
    Ok(())
}

pub fn logout() -> anyhow::Result<()> {
    let p = token_file()?;
    if p.exists() {
        fs::remove_file(p)?;
        println!("✓ Logged out");
    } else {
        println!("⚠ No token to remove");
    }
    Ok(())
}

pub async fn delete(host: String, port: u16, target: String) -> anyhow::Result<()> {
    let token = match read_token() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("✗ Not logged in");
            return Ok(());
        }
    };
    let url = format!("http://{}:{}/account/{}", host, port, target);
    let resp = Client::new().delete(&url).bearer_auth(token).send().await?;
    if resp.status().is_success() {
        println!("✓ Deleted user `{}`", target);
    } else {
        eprintln!("✗ Delete failed: {}", resp.status());
    }
    Ok(())
}

/// List all users (must be root)
pub async fn list_users(host: String, port: u16) -> anyhow::Result<()> {
    // load token
    let token = match read_token() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("✗ Not logged in");
            return Ok(());
        }
    };

    let url = format!("http://{}:{}/account/users", host, port);
    let resp = Client::new().get(&url).bearer_auth(token).send().await?;

    if !resp.status().is_success() {
        eprintln!("✗ Failed: HTTP {}", resp.status());
        return Ok(());
    }

    // Expecting JSON array of { username, role }
    let users: Vec<Value> = resp.json().await?;
    println!("{:<20} ROLE", "USERNAME");
    for u in users {
        let name = u["username"].as_str().unwrap_or("<invalid>");
        let role = u["role"].as_str().unwrap_or("<invalid>");
        println!("{:<20} {}", name, role);
    }
    Ok(())
}

```

## File: `pulson/src/logic/client/list.rs`

```rust
use crate::logic::types::{DeviceInfo, TopicInfo};
use chrono::Utc;
use reqwest::Client;

/// Format age as s/m/h/d
fn format_age(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

pub async fn run(
    host: String,
    port: u16,
    device_id: Option<String>,
    token: String,
) -> anyhow::Result<()> {
    let now = Utc::now();
    let client = Client::new();

    if let Some(dev) = device_id {
        let url = format!("http://{}:{}/devices/{}", host, port, dev);
        let resp = client.get(&url).bearer_auth(&token).send().await?;

        if !resp.status().is_success() {
            eprintln!("Error: {}", resp.text().await?);
            return Ok(());
        }

        let topics: Vec<TopicInfo> = resp.json().await?;
        println!("{:<30} {:<25} {:<10}", "TOPIC", "LAST SEEN (UTC)", "AGE");
        for t in topics {
            let secs = now.signed_duration_since(t.last_seen).num_seconds();
            println!(
                "{:<30} {:<25} {:<10}",
                t.topic,
                t.last_seen,
                format_age(secs)
            );
        }
    } else {
        let url = format!("http://{}:{}/devices", host, port);
        let resp = client.get(&url).bearer_auth(&token).send().await?;

        if !resp.status().is_success() {
            eprintln!("Error: {}", resp.text().await?);
            return Ok(());
        }

        let devices: Vec<DeviceInfo> = resp.json().await?;
        println!(
            "{:<20} {:<25} {:<10}",
            "DEVICE ID", "LAST SEEN (UTC)", "AGE"
        );
        for d in devices {
            let secs = now.signed_duration_since(d.last_seen).num_seconds();
            println!(
                "{:<20} {:<25} {:<10}",
                d.device_id,
                d.last_seen,
                format_age(secs)
            );
        }
    }

    Ok(())
}

```

## File: `pulson/src/logic/client/mod.rs`

```rust
pub mod account;
pub mod list;
pub mod ping;

```

## File: `pulson/src/logic/client/ping.rs`

```rust
use reqwest::Client;
use serde::Serialize;

#[derive(Serialize)]
struct PingPayload {
    device_id: String,
    topic: String,
}

pub async fn run(
    host: String,
    port: u16,
    device_id: String,
    topic: String,
    token: String,
) -> anyhow::Result<()> {
    let client = Client::new();
    let url = format!("http://{}:{}/ping", host, port);

    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&PingPayload { device_id, topic })
        .send()
        .await?;

    if resp.status().is_success() {
        println!("✓ Pinged {}", url);
    } else {
        eprintln!("✗ Ping failed: HTTP {}", resp.status());
    }

    Ok(())
}

```

## File: `pulson/src/logic/serve/api/account_routes.rs`

```rust
use crate::logic::serve::auth::authenticated_user;
use serde::Deserialize;
use serde_json::json;
use sled::Db;
use std::sync::Arc;
use uuid::Uuid;
use warp::{
    body::json as warp_body_json,
    http::StatusCode,
    reply::{json as warp_json, with_status},
    Filter, Rejection,
};

#[derive(Deserialize)]
struct AccountPayload {
    username: String,
    password: String,
    rootpass: Option<String>,
}

/// POST /account/register
pub fn register(
    db: Arc<Db>,
    root_pass: Option<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("account" / "register"))
        .and(warp_body_json())
        .map(move |payload: AccountPayload| {
            let user_key = format!("user:{}", payload.username);
            if db.contains_key(user_key.as_bytes()).unwrap_or(false) {
                return StatusCode::CONFLICT;
            }
            let _ = db.insert(user_key.as_bytes(), payload.password.as_bytes());

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
            let _ = db.insert(role_key.as_bytes(), role.as_bytes());
            StatusCode::CREATED
        })
}

/// POST /account/login
pub fn login(db: Arc<Db>) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
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

            match db
                .get(user_key.as_bytes())
                .ok()
                .flatten()
                .map(|v| v.to_vec())
            {
                Some(stored) if stored == payload.password.as_bytes() => {
                    let token = Uuid::new_v4().to_string();
                    let tok_key = format!("token:{}", token);
                    let _ = db.insert(tok_key.as_bytes(), payload.username.as_bytes());
                    with_status(warp_json(&json!({ "token": token })), StatusCode::OK)
                }
                _ => err(),
            }
        })
}

/// DELETE /account/<username>
pub fn delete_user(
    db: Arc<Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::delete()
        .and(warp::path!("account" / String))
        .and(auth)
        .map(move |target: String, caller: String| {
            let role_key = format!("role:{}", caller);
            if let Ok(Some(role)) = db.get(role_key.as_bytes()) {
                if &*role == b"root" {
                    let _ = db.remove(format!("user:{}", target).as_bytes());
                    let _ = db.remove(format!("role:{}", target).as_bytes());
                    return StatusCode::OK;
                }
            }
            StatusCode::FORBIDDEN
        })
}

/// GET /account/users
pub fn list_users(
    db: Arc<Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("account" / "users"))
        .and(auth)
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
}

```

## File: `pulson/src/logic/serve/api/device_routes.rs`

```rust
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

```

## File: `pulson/src/logic/serve/api/mod.rs`

```rust
pub mod account_routes;
pub mod device_routes;

use crate::logic::serve::api::account_routes::{delete_user, list_users, login, register};
use crate::logic::serve::api::device_routes::{list_all, list_one, ping};
use std::sync::Arc;
use warp::Filter;

/// Compose all account- and device-related routes into one API filter.
pub fn api_routes(
    db: Arc<sled::Db>,
    root_pass: Option<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let reg = register(db.clone(), root_pass.clone());
    let log = login(db.clone());
    let del = delete_user(db.clone());
    let list = list_users(db.clone());

    let p = ping(db.clone());
    let la = list_all(db.clone());
    let lo = list_one(db.clone());

    reg.or(log).or(del).or(list).or(p).or(lo).or(la)
}

```

## File: `pulson/src/logic/serve/auth.rs`

```rust
use std::sync::Arc;
use warp::{header::optional, reject::Reject, Filter, Rejection};

/// Marker for unauthorized rejection
#[derive(Debug)]
pub struct Unauthorized;
impl Reject for Unauthorized {}

/// A filter that extracts `Authorization: Bearer <token>` and looks up the username in sled.
pub fn authenticated_user(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    optional::<String>("authorization").and_then(move |auth: Option<String>| {
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

```

## File: `pulson/src/logic/serve/mod.rs`

```rust
pub mod api;
pub mod auth;
pub mod ui;

use crate::logic::serve::api::api_routes;
use crate::logic::serve::auth::Unauthorized;
use crate::logic::serve::ui::ui_routes;
use daemonize::Daemonize;
use shellexpand;
use sled;
use std::{net::IpAddr, path::PathBuf, sync::Arc};
use warp::{Filter, Rejection};

pub async fn run(
    host: String,
    port: u16,
    db_path: String,
    daemon: bool,
    root_pass: Option<String>,
    _webui: bool,
) -> anyhow::Result<()> {
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

    // 3) Build API routes
    let api = api_routes(db.clone(), root_pass.clone())
        .recover(|err: Rejection| async move {
            if err.find::<Unauthorized>().is_some() {
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({ "error": "Unauthorized" })),
                    warp::http::StatusCode::UNAUTHORIZED,
                ))
            } else {
                Err(err)
            }
        })
        .boxed();

    // 4) Build UI routes (static + SPA)
    let ui = ui_routes().boxed();

    // 5) Combine and serve
    let routes = api.or(ui);
    println!("pulson server running on http://{}:{}", host, port);
    let ip: IpAddr = host.parse()?;
    warp::serve(routes).run((ip, port)).await;
    Ok(())
}

```

## File: `pulson/src/logic/serve/ui.rs`

```rust
use mime_guess;
use rust_embed::RustEmbed;
use warp::{http::Response, Filter, Rejection};

#[derive(RustEmbed)]
#[folder = "../pulson-ui/ui/dist"]
struct Asset;

/// Serves static assets under `/static/<path>` and falls back to `index.html` for SPA.
pub fn ui_routes() -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
    // 1) static files
    let static_files = warp::get()
        .and(warp::path("static"))
        .and(warp::path::tail())
        .map(|tail: warp::path::Tail| {
            let path = tail.as_str();
            if let Some(content) = Asset::get(path) {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header("content-type", mime.as_ref())
                    .body(content.data.into_owned())
            } else {
                Response::builder().status(404).body("Not Found".into())
            }
        });

    // 2) SPA fallback
    let spa = warp::get().and(warp::path::full()).map(|_| {
        let file = Asset::get("index.html").expect("index.html missing");
        Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(file.data.into_owned())
    });

    static_files.or(spa)
}

```

## File: `pulson/src/logic/mod.rs`

```rust
pub mod client;
pub mod serve;
pub mod types;

```

## File: `pulson/src/logic/types.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device ↔ last_seen summary
#[derive(Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_seen: DateTime<Utc>,
}

/// Topic ↔ last_seen summary
#[derive(Serialize, Deserialize)]
pub struct TopicInfo {
    pub topic: String,
    pub last_seen: DateTime<Utc>,
}

```

## File: `pulson/src/cli.rs`

```rust
use clap::{Parser, Subcommand};
use client::{account, list, ping};

/// realtime system/robot monitoring and tracing
#[derive(Parser)]
#[command(name = "pulson")]
pub struct Cli {
    /// Address to bind (serve) or connect to (client)
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to bind or connect to
    #[arg(short, long, default_value_t = 3030)]
    pub port: u16,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the HTTP server
    Serve {
        /// Path to database file (supports `~`)
        #[arg(short, long, default_value = "~/.local/share/pulson")]
        db_path: String,
        /// Run as daemon in background (Unix only)
        #[arg(long)]
        daemon: bool,
        /// Optional shared secret to create root users
        #[arg(long, alias = "rootpass")]
        root_pass: Option<String>,
        /// Also serve the Web UI from `pulson-ui/ui/dist`
        #[arg(long)]
        webui: bool,
    },

    /// Query the running server for all tracked devices (or topics for one)
    List {
        #[arg(value_name = "DEVICE_ID")]
        device_id: Option<String>,
    },

    /// Send a ping for a given device_id and topic
    Ping {
        #[arg(short = 'd', long)]
        device_id: String,
        #[arg(short = 't', long)]
        topic: String,
    },

    /// User account management (register, login, logout, delete, list)
    Account {
        #[command(subcommand)]
        action: AccountAction,
    },
}

#[derive(Subcommand)]
pub enum AccountAction {
    Register {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
        /// Supply `--root-pass` (or `--rootpass`) to become root
        #[arg(long, alias = "root-pass")]
        rootpass: Option<String>,
    },
    Login {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
    },
    Logout,
    Delete {
        #[arg(value_name = "USERNAME")]
        username: String,
    },
    List,
}

```

## File: `pulson/src/main.rs`

```rust
mod cli;
mod logic;

use clap::Parser;
use cli::{AccountAction, Cli, Commands};
use logic::account::read_token;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // Allow PULSON_IP / PULSON_PORT env vars to override flags
    let host = std::env::var("PULSON_IP").unwrap_or_else(|_| args.host.clone());
    let port = std::env::var("PULSON_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(args.port);

    // Pre‐load token for protected commands
    let token = match &args.command {
        Commands::Serve { .. } => None,
        Commands::Account { .. } => None,
        Commands::List { .. } | Commands::Ping { .. } => match read_token() {
            Ok(t) => Some(t),
            Err(_) => {
                eprintln!("✗ Not logged in: please run `pulson account login` first");
                return Ok(());
            }
        },
    };

    match args.command {
        Commands::Serve {
            db_path,
            daemon,
            root_pass,
            webui,
        } => logic::serve::run(host.clone(), port, db_path, daemon, root_pass, webui).await?,

        Commands::List { device_id } => {
            logic::list::run(host.clone(), port, device_id, token.clone().unwrap()).await?
        }

        Commands::Ping { device_id, topic } => {
            logic::ping::run(host.clone(), port, device_id, topic, token.clone().unwrap()).await?
        }

        Commands::Account { action } => match action {
            AccountAction::Register {
                username,
                password,
                rootpass,
            } => logic::account::register(host.clone(), port, username, password, rootpass).await?,
            AccountAction::Login { username, password } => {
                logic::account::login(host.clone(), port, username, password).await?
            }
            AccountAction::Logout => logic::account::logout()?,
            AccountAction::Delete { username } => {
                logic::account::delete(host.clone(), port, username).await?
            }
            AccountAction::List => logic::account::list_users(host.clone(), port).await?,
        },
    }

    Ok(())
}

```

## File: `pulson/Cargo.toml`

```toml
[package]
name = "pulson"
version = "0.0.7"
authors = ["Trim Bresilla <trim.bresilla@gmail.com>"]
description = "realtime system/robot monitoring and tracing"
edition = "2021"
license = "MIT"
build = "build.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
warp = "0.3"
clap = { version = "4.1", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sled = "0.34"
chrono = { version = "0.4", features = ["serde"] }
daemonize = "0.5"
anyhow = "1.0"
shellexpand = "2.1"
directories = "4.0"
uuid        = { version = "1", features = ["v4"] }
reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }

rust-embed = "6.3"
mime_guess = "2.0"

```

## File: `pulson/build.rs`

```rust
// pulson/build.rs

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../pulson-ui/src");
    println!("cargo:rerun-if-changed=../pulson-ui/static/index.html");

    // 1) Get the crate root as a PathBuf
    let pulson_manifest = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let pulson_root = PathBuf::from(&pulson_manifest);

    // 2) Compute the UI directory (sibling of pulson/)
    let parent_dir = pulson_root
        .parent()
        .expect("pulson has no parent directoryy");
    let ui_dir = parent_dir.join("pulson-ui");
    let dist_dir = ui_dir.join("ui").join("dist");
    let static_index = ui_dir.join("static").join("index.html");
    let dist_index = dist_dir.join("index.html");

    // 3) Build the UI via wasm-pack
    let status = Command::new("wasm-pack")
        .env("CARGO_TARGET_DIR", "../target/target-wasm")
        .args(&[
            "build",
            "../pulson-ui",
            "--release",
            "--target",
            "web",
            "--out-dir",
            "ui/dist",
        ])
        .current_dir(&ui_dir)
        .status()
        .expect("failed to run wasm-pack");
    if !status.success() {
        panic!("wasm-pack build failed");
    }

    fs::copy(static_index, dist_index).expect("failed to copy index.html");
}

```

