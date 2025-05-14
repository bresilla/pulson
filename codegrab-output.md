# Project Structure

```
./
├── pulson/
│   ├── src/
│   │   ├── logic/
│   │   │   ├── account.rs
│   │   │   ├── list.rs
│   │   │   ├── mod.rs
│   │   │   ├── ping.rs
│   │   │   ├── serve.rs
│   │   │   └── types.rs
│   │   ├── cli.rs
│   │   └── main.rs
│   └── Cargo.toml
└── pulson-ui/
    ├── src/
    │   └── lib.rs
    ├── ui/
    │   └── dist/
    │       └── index.html
    └── Cargo.toml
```

# Project Files

## File: `pulson/src/logic/account.rs`

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

## File: `pulson/src/logic/list.rs`

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

## File: `pulson/src/logic/mod.rs`

```rust
pub mod account;
pub mod list;
pub mod ping;
pub mod serve;
pub mod types;

```

## File: `pulson/src/logic/ping.rs`

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

## File: `pulson/src/logic/serve.rs`

```rust
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

pulson-ui    = { path = "../pulson-ui" }

```

## File: `pulson-ui/src/lib.rs`

```rust
use warp::Filter;

/// Serves `ui/dist/index.html` at `/` and all other files under `ui/dist/`.
pub fn ui_routes() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    // GET  /         → index.html
    let index = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("pulson-ui/ui/dist/index.html"));

    // GET  /<file>   → pulson-ui/ui/dist/<file>
    let static_dir = warp::get().and(warp::fs::dir("pulson-ui/ui/dist"));

    index.or(static_dir)
}

```

## File: `pulson-ui/ui/dist/index.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Pulson Dashboard</title>
  <script type="module" src="./pkg/app.js"></script>
</head>
<body>
  <h1>Welcome to Pulson Dashboard</h1>
  <div id="app"></div>
</body>
</html>

```

## File: `pulson-ui/Cargo.toml`

```toml
[package]
name = "pulson-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
warp = "0.3"


```

