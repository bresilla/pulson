mod cli;
mod logic;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Serve {
            host,
            port,
            db_path,
            daemon,
        } => logic::serve::run(host, port, db_path, daemon).await?,
        Commands::List { host, port } => logic::list::run(host, port).await?,
        Commands::Ping {
            host,
            port,
            device_id,
        } => logic::ping::run(host, port, device_id).await?,
    }

    Ok(())
}
