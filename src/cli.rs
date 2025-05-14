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
        /// Optional shared secret to create root users
        #[arg(long)]
        root_pass: Option<String>,
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

    /// User account management (register, login, logout, delete)
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
        /// If you have the server’s root_pass, supply it here to become root
        #[arg(long)]
        root_pass: Option<String>,
    },
    /// Login with username/password (saves token locally)
    Login {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
    },
    /// Remove saved token
    Logout,
    /// Delete another user (root only)
    Delete {
        /// Username to delete
        #[arg(value_name = "USERNAME")]
        username: String,
    },
    /// List all users (root only)
    List,
}
