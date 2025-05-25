// pulson/src/main.rs

mod cli;
mod logic;
mod database;

use clap::Parser;
use cli::{AccountAction, Cli, Commands};
use logic::client::{account, list, ping};

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
        Commands::List { .. } | Commands::Ping { .. } => match account::read_token() {
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
        } => {
            // Run the HTTP server
            logic::serve::run(host, port, db_path, daemon, root_pass, webui).await?
        }

        Commands::List { 
            device_id,
            format,
            sort,
            status,
            watch,
            interval,
            extended,
        } => {
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
            ).await?
        }

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
                AccountAction::Logout => account::logout()?,
                AccountAction::Delete { username } => account::delete(host, port, username).await?,
                AccountAction::List => account::list_users(host, port).await?,
            }
        }
    }

    Ok(())
}
