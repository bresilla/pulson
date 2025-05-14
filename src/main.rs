mod cli;
mod logic;

use clap::Parser;
use cli::{AccountAction, Cli, Commands};
use logic::account::read_token;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // Pre‐load token for everything except `serve` and `account`
    let token = match &args.command {
        Commands::Serve { .. } => None,
        Commands::Account { .. } => None,
        Commands::List { .. } | Commands::Ping { .. } => match read_token() {
            Ok(t) => Some(t),
            Err(_) => {
                eprintln!("✗ No token found: please run `pulson account login` first");
                return Ok(());
            }
        },
    };

    match args.command {
        Commands::Serve { db_path, daemon } => {
            logic::serve::run(args.host, args.port, db_path, daemon).await?
        }

        Commands::List { device_id } => {
            logic::list::run(args.host, args.port, device_id, token.unwrap()).await?
        }

        Commands::Ping { device_id, topic } => {
            logic::ping::run(args.host, args.port, device_id, topic, token.unwrap()).await?
        }

        Commands::Account { action } => match action {
            AccountAction::Register { username, password } => {
                logic::account::register(args.host, args.port, username, password).await?
            }
            AccountAction::Login { username, password } => {
                logic::account::login(args.host, args.port, username, password).await?
            }
            AccountAction::Logout => logic::account::logout()?,
        },
    }

    Ok(())
}
