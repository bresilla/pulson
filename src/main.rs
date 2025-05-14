mod cli;
mod logic;

use clap::Parser;
use cli::{AccountAction, Cli, Commands};
use logic::account::read_token;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // Allow PULSON_IP / PULSON_PORT to override flags
    let host = std::env::var("PULSON_IP").unwrap_or_else(|_| args.host.clone());
    let port = std::env::var("PULSON_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(args.port);

    // Pre‐load token for protected commands (List, Ping, Account::Delete/List)
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
        } => logic::serve::run(host.clone(), port, db_path, daemon, root_pass).await?,

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
                root_pass,
            } => {
                logic::account::register(host.clone(), port, username, password, root_pass).await?
            }
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
