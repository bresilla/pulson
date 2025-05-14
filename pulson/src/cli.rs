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
        #[arg(long, alias = "rootpass")]
        root_pass: Option<String>,
        /// Also serve the Web UI from `pulson-ui/ui/dist`
        #[arg(long)]
        webui: bool,
    },

    /// Query the running server for all tracked devices (or topics for one)
    List {
        #[arg(value_name = "DEVICE_ID")]
        device_id: Option<String>,
    },

    /// Send a ping for a given device_id and topic
    Ping {
        #[arg(short = 'd', long)]
        device_id: String,
        #[arg(short = 't', long)]
        topic: String,
    },

    /// User account management (register, login, logout, delete, list)
    Account {
        #[command(subcommand)]
        action: AccountAction,
    },
}

#[derive(Subcommand)]
pub enum AccountAction {
    Register {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
        /// Supply `--root-pass` (or `--rootpass`) to become root
        #[arg(long, alias = "root-pass")]
        rootpass: Option<String>,
    },
    Login {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
    },
    Logout,
    Delete {
        #[arg(value_name = "USERNAME")]
        username: String,
    },
    List,
}
