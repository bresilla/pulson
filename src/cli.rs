use clap::{Parser, Subcommand};

/// realtime system/robot monitoring and tracing
#[derive(Parser)]
#[command(name = "pulson")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the HTTP server
    Serve {
        /// Address to bind (e.g. 127.0.0.1)
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 3030)]
        port: u16,

        /// Path to database file (supports `~`)
        #[arg(short, long, default_value = "~/.local/share/pulson")]
        db_path: String,

        /// Run as daemon in background (Unix only)
        #[arg(long)]
        daemon: bool,
    },

    /// Query the running server for all tracked devices (or topics for one)
    List {
        /// Address where pulson is running
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        /// Port where pulson is listening
        #[arg(short, long, default_value_t = 3030)]
        port: u16,

        /// If provided, show topics just for this device
        #[arg(value_name = "DEVICE_ID")]
        device_id: Option<String>,
    },

    /// Send a ping (POST /ping) for a given device_id and topic
    Ping {
        /// Address where pulson is running
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        /// Port where pulson is listening
        #[arg(short, long, default_value_t = 3030)]
        port: u16,

        /// Device ID to ping
        #[arg(short = 'd', long)]
        device_id: String,

        /// Topic for this ping (slash‐separated)
        #[arg(short = 't', long)]
        topic: String,
    },
}
