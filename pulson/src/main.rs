// pulson/src/main.rs

mod cli;
mod logic;

use clap::Parser;
use cli::{AccountAction, Cli, Commands, DeviceAction, ConfigAction};
use logic::client::{account, list, ping, device, config};
use logic::config::StatusConfig;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command‐line arguments
    let args = Cli::parse();

    // Allow PULSON_IP / PULSON_PORT to override flags
    let host = std::env::var("PULSON_IP").unwrap_or_else(|_| args.host.clone());
    let port = std::env::var("PULSON_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(args.port);

    // Pre‐load token for client commands (List & Ping)
    let token = match &args.command {
        Commands::Serve { .. } => None,
        Commands::Account { .. } => None,
        Commands::Config { .. } => None, // Config commands work with local files, no auth needed
        Commands::Device { .. } | Commands::Ping { .. } => match account::read_token() {
            Ok(t) => Some(t),
            Err(_) => {
                eprintln!("✗ Not logged in: please run `pulson account login` first`");
                return Ok(());
            }
        },
    };

    // Dispatch on subcommand
    match args.command {
        Commands::Serve {
            db_path,
            daemon,
            root_pass,
            webui,
            config,
            online_threshold,
            warning_threshold,
            stale_threshold,
        } => {
            // Create configuration from file and CLI arguments
            let status_config = if let Some(config_path) = config {
                // Load from file first, then override with CLI args
                let base_config = StatusConfig::from_file(&config_path)?;
                StatusConfig {
                    online_threshold_seconds: online_threshold.unwrap_or(base_config.online_threshold_seconds),
                    warning_threshold_seconds: warning_threshold.unwrap_or(base_config.warning_threshold_seconds),
                    stale_threshold_seconds: stale_threshold.unwrap_or(base_config.stale_threshold_seconds),
                }
            } else {
                // Use CLI args and environment variables
                StatusConfig::from_args_and_env(online_threshold, warning_threshold, stale_threshold)
            };

            // Wrap configuration in Arc<Mutex<>> for thread-safe sharing
            let status_config = Arc::new(Mutex::new(status_config));

            // Run the HTTP server
            logic::serve::run(host, port, db_path, daemon, root_pass, webui, status_config).await?
        }

        Commands::Device { action } => match action {
            DeviceAction::List {
                device_id,
                format,
                sort,
                status,
                watch,
                interval,
                extended,
                config,
                online_threshold,
                warning_threshold,
                stale_threshold,
            } => {
                // Create configuration from file and CLI arguments
                let status_config = if let Some(config_path) = config {
                    // Load from file first, then override with CLI args
                    let base_config = StatusConfig::from_file(&config_path)?;
                    StatusConfig {
                        online_threshold_seconds: online_threshold.unwrap_or(base_config.online_threshold_seconds),
                        warning_threshold_seconds: warning_threshold.unwrap_or(base_config.warning_threshold_seconds),
                        stale_threshold_seconds: stale_threshold.unwrap_or(base_config.stale_threshold_seconds),
                    }
                } else {
                    // Use CLI args and environment variables
                    StatusConfig::from_args_and_env(online_threshold, warning_threshold, stale_threshold)
                };

                // Client: list devices or topics
                list::run(
                    host,
                    port,
                    device_id,
                    token.unwrap(),
                    format,
                    sort,
                    status,
                    watch,
                    interval,
                    extended,
                    status_config,
                )
                .await?
            }
            DeviceAction::Delete { device_id } => {
                // Placeholder for delete device logic
                // println!("Deleting device: {}", device_id);
                // TODO: Implement actual device deletion logic e.g.:
                device::delete(host, port, device_id, token.unwrap()).await?
            }
        },

        Commands::Ping { device_id, topic } => {
            // Client: send a ping
            ping::run(host, port, device_id, topic, token.unwrap()).await?
        }

        Commands::Account { action } => {
            // Client: account management
            match action {
                AccountAction::Register {
                    username,
                    password,
                    rootpass,
                } => account::register(host, port, username, password, rootpass).await?,
                AccountAction::Login { username, password } => {
                    account::login(host, port, username, password).await?
                }
                AccountAction::Logout => account::logout(host, port).await?, // Modified this line
                AccountAction::Delete { username } => account::delete(host, port, username).await?,
                AccountAction::List => account::list_users(host, port).await?,
            }
        }

        Commands::Config { action } => {
            // Configuration management
            match action {
                ConfigAction::Show { config } => {
                    config::show(config).await?
                }
                ConfigAction::Set {
                    config,
                    online_threshold,
                    warning_threshold,
                    stale_threshold,
                } => {
                    config::set(config, online_threshold, warning_threshold, stale_threshold).await?
                }
            }
        }
    }

    Ok(())
}
