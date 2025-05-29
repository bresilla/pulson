use clap::{Parser, ValueEnum, Subcommand};
use std::str::FromStr;

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
pub enum DataType {
    Pulse,
    Gps,
    Sensor,
    Trigger,
    Event,
    Image,
}

/// realtime system/robot monitoring and tracing
#[derive(Parser)]
#[command(name = "pulson")]
pub struct Cli {
    /// Bind address: e.g., 127.0.0.1:3030, 0.0.0.0:8080, https://sub.domain.com, http://localhost:3030 
    #[arg(short = 'H', long, default_value = "127.0.0.1:3030", env = "PULSON_HOST")]
    pub host: String,

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

    /// Send a pulse with specific data type
    Pulse {
        /// Device identifier
        #[arg(short = 'd', long)]
        device_id: String,
        /// Topic for the pulse
        #[arg(short = 't', long)]
        topic: String,
        /// Data type to send (default: pulse)
        #[arg(long, default_value = "pulse")]
        data_type: DataType,
        /// Custom JSON data payload (overrides data type defaults)
        #[arg(value_name = "JSON_DATA")]
        data: Option<String>,
        /// Latitude for GPS data type
        #[arg(long, requires = "longitude")]
        latitude: Option<f64>,
        /// Longitude for GPS data type  
        #[arg(long, requires = "latitude")]
        longitude: Option<f64>,
        /// Altitude for GPS data type (optional)
        #[arg(long)]
        altitude: Option<f64>,
        /// Sensor value for sensor data type
        #[arg(long)]
        value: Option<f64>,
        /// State for trigger data type (true/false)
        #[arg(long)]
        state: Option<bool>,
        /// Message for event data type
        #[arg(long)]
        message: Option<String>,
        /// Image width for image data type
        #[arg(long)]
        width: Option<u32>,
        /// Image height for image data type
        #[arg(long)]
        height: Option<u32>,
        /// Path to image file for image data type
        #[arg(long)]
        image_file: Option<String>,
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

impl Cli {
    /// Parse the host parameter and return connection details
    pub fn parse_host(&self) -> HostConfig {
        HostConfig::from_str(&self.host).unwrap_or_else(|e| {
            eprintln!("Error parsing host '{}': {}", self.host, e);
            std::process::exit(1);
        })
    }
}

#[derive(Debug, Clone)]
pub struct HostConfig {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub full_url: String,
    pub bind_address: String,
}

impl FromStr for HostConfig {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Check if it's a full URL
        if s.starts_with("http://") || s.starts_with("https://") {
            let url = url::Url::parse(s).map_err(|e| format!("Invalid URL: {}", e))?;
            
            let scheme = url.scheme().to_string();
            let host = url.host_str().ok_or("Missing host in URL")?.to_string();
            let port = url.port().unwrap_or(match scheme.as_str() {
                "https" => 443,
                "http" => 80,
                _ => return Err("Unsupported scheme".to_string()),
            });
            
            let full_url = s.to_string();
            let bind_address = format!("{}:{}", 
                if host == "localhost" { "127.0.0.1" } else { &host }, 
                port
            );

            Ok(HostConfig {
                scheme,
                host,
                port,
                full_url,
                bind_address,
            })
        } else {
            // Assume it's host:port format
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 2 {
                return Err("Host must be in format 'host:port' or a full URL".to_string());
            }

            let host = parts[0].to_string();
            let port: u16 = parts[1].parse().map_err(|_| "Invalid port number")?;
            
            // Default to http for local addresses
            let scheme = "http".to_string();
            let full_url = format!("{}://{}:{}", scheme, host, port);
            let bind_address = s.to_string();

            Ok(HostConfig {
                scheme,
                host,
                port,
                full_url,
                bind_address,
            })
        }
    }
}

impl HostConfig {
    /// Get base_url for client connections (if using full URL) or None for host:port
    pub fn base_url(&self) -> Option<String> {
        if self.full_url.starts_with("https://") || 
           (self.full_url.starts_with("http://") && self.port != 3030) {
            Some(self.full_url.clone())
        } else {
            None
        }
    }

    /// Get the bind address for server listening (IP:port format)
    pub fn server_bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Get the full server URL for display purposes
    pub fn server_url(&self) -> String {
        if self.full_url.starts_with("http://") || self.full_url.starts_with("https://") {
            self.full_url.clone()
        } else {
            format!("{}://{}", self.scheme, self.bind_address)
        }
    }

    /// Get the URL scheme (http or https) - available for future use
    #[allow(dead_code)]
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Check if this is an HTTPS configuration - available for future use
    #[allow(dead_code)]
    pub fn is_https(&self) -> bool {
        self.scheme == "https"
    }
}
