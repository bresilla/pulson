// pulson/src/main.rs

mod cli;
mod logic;

use clap::Parser;
use cli::{AccountAction, Cli, Commands, DeviceAction, ConfigAction};
use crate::logic::client::{account, list, pulse, device};
use crate::logic::client::config::{show, set}; // Import show and set directly using crate path
use logic::config::StatusConfig;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command‐line arguments
    let args = Cli::parse();

    // Parse the host configuration
    let host_config = args.parse_host();

    // Pre‐load token for client commands (List & Ping)
    let token = match &args.command {
        Commands::Serve { .. } => None,
        Commands::Account { .. } => None,
        Commands::Config { .. } => None, // Config commands work with local files, no auth needed
        Commands::Device { .. } | Commands::Pulse { .. } => match account::read_token() {
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
            config: _,  // Config file option ignored - using purely server-based configuration
            online_threshold,
            warning_threshold,
            stale_threshold,
        } => {
            // Create configuration from CLI arguments and environment variables only
            let status_config = StatusConfig::from_args_and_env(online_threshold, warning_threshold, stale_threshold);

            // Wrap configuration in Arc<Mutex<>> for thread-safe sharing
            let status_config = Arc::new(Mutex::new(status_config));

            // Run the HTTP server - use host_config for server
            logic::serve::run(host_config, db_path, daemon, root_pass, webui, status_config).await?
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
                config: _,  // Config file option ignored - using purely server-based configuration
                online_threshold,
                warning_threshold,
                stale_threshold,
            } => {
                // Create configuration from CLI arguments and environment variables only
                let status_config = StatusConfig::from_args_and_env(online_threshold, warning_threshold, stale_threshold);

                // Client: list devices or topics
                list::run(
                    host_config.base_url(),
                    host_config.host.clone(),
                    host_config.port,
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
                device::delete(host_config.base_url(), host_config.host, host_config.port, device_id, token.unwrap()).await?
            }
        },

        Commands::Pulse { 
            device_id, 
            topic, 
            data_type,
            data,
            latitude,
            longitude, 
            altitude,
            value,
            state,
            message,
            width,
            height,
        } => {
            // Client: send a unified pulse (ping or data)
            pulse::run(
                host_config.base_url(),
                host_config.host, 
                host_config.port, 
                device_id, 
                topic, 
                data_type,
                data,
                latitude,
                longitude,
                altitude,
                value,
                state,
                message,
                width,
                height,
                token.unwrap()
            ).await?
        }

        Commands::Account { action } => {
            // Client: account management
            match action {
                AccountAction::Register {
                    username,
                    password,
                    rootpass,
                } => account::register(host_config.base_url(), host_config.host, host_config.port, username, password, rootpass).await?,
                AccountAction::Login { username, password } => {
                    account::login(host_config.base_url(), host_config.host, host_config.port, username, password).await?
                }
                AccountAction::Logout => account::logout(host_config.base_url(), host_config.host, host_config.port).await?,
                AccountAction::Delete { username } => account::delete(host_config.base_url(), host_config.host, host_config.port, username).await?,
                AccountAction::List => account::list_users(host_config.base_url(), host_config.host, host_config.port).await?,
            }
        }

        Commands::Config { action } => {
            // Configuration management
            match action {
                ConfigAction::Show => {
                    show().await? // Call imported show function
                }
                ConfigAction::Set {
                    online_threshold,
                    warning_threshold,
                    stale_threshold,
                } => {
                    set(online_threshold, warning_threshold, stale_threshold).await? // Call imported set function
                }
            }
        }
    }

    Ok(())
}
