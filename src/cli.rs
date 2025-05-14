use clap::{Parser, Subcommand};

/// realtime system/robot monitoring and tracing
#[derive(Parser)]
#[command(name = "pulson")]
pub struct Cli {
    /// Address to bind (serve) or connect to (client)
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to bind or connect to
    #[arg(short, long, default_value_t = 3030)]
    pub port: u16,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the HTTP server
    Serve {
        /// Path to database file (supports `~`)
        #[arg(short, long, default_value = "~/.local/share/pulson")]
        db_path: String,
        /// Run as daemon in background (Unix only)
        #[arg(long)]
        daemon: bool,
    },

    /// Query the running server for all tracked devices (or topics for one)
    List {
        /// If provided, show topics just for this device
        #[arg(value_name = "DEVICE_ID")]
        device_id: Option<String>,
    },

    /// Send a ping (POST /ping) for a given device_id and topic
    Ping {
        /// Device ID to ping
        #[arg(short = 'd', long)]
        device_id: String,
        /// Topic for this ping (slash-separated)
        #[arg(short = 't', long)]
        topic: String,
    },

    /// User account management (register, login, logout)
    Account {
        #[command(subcommand)]
        action: AccountAction,
    },
}

#[derive(Subcommand)]
pub enum AccountAction {
    /// Register a new username/password
    Register {
        /// Username to register
        #[arg(short, long)]
        username: String,
        /// Password for the new account
        #[arg(short, long)]
        password: String,
    },
    /// Login and save authentication token
    Login {
        /// Username
        #[arg(short, long)]
        username: String,
        /// Password
        #[arg(short, long)]
        password: String,
    },
    /// Logout and remove saved token
    Logout,
}
