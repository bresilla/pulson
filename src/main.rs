mod cli;
mod logic;

use clap::Parser;
use cli::{AccountAction, Cli, Commands};
use logic::account::read_token;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // Pre‐load token for protected commands (List, Ping)
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
        } => {
            // Pass root_pass into serve::run
            logic::serve::run(args.host, args.port, db_path, daemon, root_pass).await?
        }

        Commands::List { device_id } => {
            logic::list::run(args.host, args.port, device_id, token.unwrap()).await?
        }

        Commands::Ping { device_id, topic } => {
            logic::ping::run(args.host, args.port, device_id, topic, token.unwrap()).await?
        }

        Commands::Account { action } => match action {
            AccountAction::Register {
                username,
                password,
                rootpass,
            } => {
                logic::account::register(args.host, args.port, username, password, rootpass).await?
            }
            AccountAction::Login { username, password } => {
                logic::account::login(args.host, args.port, username, password).await?
            }
            AccountAction::Logout => logic::account::logout()?,
            AccountAction::Delete { username } => {
                logic::account::delete(args.host, args.port, username).await?
            }
        },
    }

    Ok(())
}
