mod cli;
mod logic;

use clap::Parser;
use cli::{AccountAction, Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Serve { db_path, daemon } => {
            logic::serve::run(args.host, args.port, db_path, daemon).await?
        }

        Commands::List { device_id } => logic::list::run(args.host, args.port, device_id).await?,

        Commands::Ping { device_id, topic } => {
            logic::ping::run(args.host, args.port, device_id, topic).await?
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
