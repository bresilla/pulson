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
        Commands::List {
            host,
            port,
            device_id,
        } => logic::list::run(host, port, device_id).await?,
        Commands::Ping {
            host,
            port,
            device_id,
            topic,
        } => logic::ping::run(host, port, device_id, topic).await?,
    }

    Ok(())
}
