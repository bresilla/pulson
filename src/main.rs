mod cli;
mod logic;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // now host & port are in `args.host` / `args.port` for every command
    match args.command {
        Commands::Serve { db_path, daemon } => {
            logic::serve::run(args.host, args.port, db_path, daemon).await?
        }
        Commands::List { device_id } => logic::list::run(args.host, args.port, device_id).await?,
        Commands::Ping { device_id, topic } => {
            logic::ping::run(args.host, args.port, device_id, topic).await?
        }
    }

    Ok(())
}
