use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Compact,
}

#[derive(Clone, ValueEnum)]
pub enum SortBy {
    LastSeen,
    Name,
    Status,
    TopicCount,
    PingCount,
}

#[derive(Clone, ValueEnum)]
pub enum StatusFilter {
    Online,
    Warning,
    Offline,
    Active,
    Recent,
    Stale,
    Inactive,
}

#[derive(Clone, ValueEnum)]
pub enum DataType {
    Map,  // For GNSS/GPS coordinates 
    Sensor, // For sensor readings
    Event,  // For event data
    Message, // For text messages
}

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
        /// Path to configuration file (supports `~`)
        #[arg(short, long)]
        config: Option<String>,
        /// Online threshold in seconds (overrides config file)
        #[arg(long)]
        online_threshold: Option<u64>,
        /// Warning threshold in seconds (overrides config file)  
        #[arg(long)]
        warning_threshold: Option<u64>,
        /// Stale threshold in seconds (overrides config file)
        #[arg(long)]
        stale_threshold: Option<u64>,
    },

    /// Device management (list, delete)
    Device {
        #[command(subcommand)]
        action: DeviceAction,
    },

    /// Send a ping for a given device_id and topic
    Ping {
        #[arg(short = 'd', long)]
        device_id: String,
        #[arg(short = 't', long)]
        topic: String,
    },

    /// Send structured data for a given device_id and topic
    Data {
        /// Type of data being sent
        #[arg(long)]
        r#type: DataType,
        /// Device identifier
        #[arg(short = 'd', long)]
        device_id: String,
        /// Topic for the data
        #[arg(short = 't', long)]
        topic: String,
        /// JSON data payload
        #[arg(value_name = "JSON_DATA")]
        data: String,
    },

    /// User account management (register, login, logout, delete, list)
    Account {
        #[command(subcommand)]
        action: AccountAction,
    },

    /// Configuration management (view, set thresholds)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum DeviceAction {
    /// Query the running server for all tracked devices (or topics for one)
    List {
        #[arg(value_name = "DEVICE_ID")]
        device_id: Option<String>,
        /// Output format: table (default), json, or compact
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        /// Sort by: last-seen (default), name, status, topic-count, ping-count
        #[arg(short, long, default_value = "last-seen")]
        sort: SortBy,
        /// Show only devices/topics with specific status
        #[arg(long)]
        status: Option<StatusFilter>,
        /// Watch mode: continuously update the listing
        #[arg(short, long)]
        watch: bool,
        /// Watch interval in seconds (default: 5)
        #[arg(long, default_value_t = 5)]
        interval: u64,
        /// Show extended information (descriptions, counts, etc.)
        #[arg(short = 'x', long)]
        extended: bool,
        /// Path to configuration file (supports `~`)
        #[arg(short, long)]
        config: Option<String>,
        /// Online threshold in seconds (overrides config file)
        #[arg(long)]
        online_threshold: Option<u64>,
        /// Warning threshold in seconds (overrides config file)  
        #[arg(long)]
        warning_threshold: Option<u64>,
        /// Stale threshold in seconds (overrides config file)
        #[arg(long)]
        stale_threshold: Option<u64>,
    },
    /// Delete a device by its ID
    Delete {
        #[arg(value_name = "DEVICE_ID")]
        device_id: String,
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

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration and thresholds
    Show,
    /// Set device status thresholds
    Set {
        /// Online threshold in seconds
        #[arg(long)]
        online_threshold: Option<u64>,
        /// Warning threshold in seconds  
        #[arg(long)]
        warning_threshold: Option<u64>,
        /// Stale threshold in seconds
        #[arg(long)]
        stale_threshold: Option<u64>,
    },
}
