<img align="right" width="26%" src="./book/src/images/logo.png">

# Pulson

**Real-time system/robot monitoring and tracing with Progressive Web App dashboard**

[![Version](https://img.shields.io/badge/version-0.3.3-blue.svg)](./CHANGELOG.md)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE.md)

Pulson is a comprehensive monitoring solution for IoT devices, robots, and distributed systems. It provides real-time data collection, visualization, and alerting through both a CLI interface and a modern web dashboard with Progressive Web App (PWA) capabilities.

## 🚀 Features

### Core Capabilities
- **📊 Multi-Type Data Support**: Handle 6 different data types (pulse, GPS, sensor, trigger, event, image)
- **🌐 Real-Time Dashboard**: Modern web interface with live updates every 5 seconds
- **📱 Progressive Web App**: Install as a mobile app with offline support
- **🔐 User Authentication**: JWT-based authentication with role-based access control
- **⚙️ Flexible Configuration**: CLI args, environment variables, and config files
- **🗃️ SQLite Database**: Lightweight, embedded database with user isolation
- **🚀 WASM Performance**: Rust + WebAssembly for optimal performance

### Data Types & Visualizations
- **Pulse**: Simple heartbeat/ping monitoring with timeline visualization
- **GPS**: Location tracking with interactive maps
- **Sensor**: Numeric data with real-time charts and threshold monitoring
- **Trigger**: Boolean state changes with state timeline
- **Event**: Text messages and system events with filterable logs
- **Image**: Visual data with gallery view and metadata

### Progressive Web App Features
- **📱 Mobile Installation**: Install directly to device home screen
- **🔄 Offline Support**: Basic functionality when disconnected
- **⚡ Service Worker**: Fast loading with intelligent caching
- **🎨 Native Experience**: App-like interface with proper theming

## 📖 Quick Start

### Installation

```bash
# Install Rust and dependencies
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Install WASM tools
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
rustup target add wasm32-unknown-unknown

# Build Pulson
cargo build --release
```

### Basic Usage

```bash
# Start the server
pulson --host 127.0.0.1:3030 serve --db-path ~/.local/share/pulson

# Register a user
pulson --host 127.0.0.1:3030 account register --username myuser --password mypass

# Send data
pulson --host 127.0.0.1:3030 pulse --device-id robot1 --topic sensors --data-type sensor --value 23.5

# View dashboard at http://127.0.0.1:3030
```

## 🖥️ Command Line Interface

```bash
pulson --help

realtime system/robot monitoring and tracing

Usage: pulson [OPTIONS] <COMMAND>

Commands:
  serve         Run the HTTP server
  device        Device management (list, delete)
  pulse         Send a pulse with specific data type
  account       User account management (register, login, logout, delete, list)
  config        Configuration management (show, set thresholds)
  help          Print this message or the help of the given subcommand(s)

Options:
  -H, --host <HOST>   Bind address: e.g., 127.0.0.1:3030, 0.0.0.0:8080, https://sub.domain.com, http://localhost:3030
                      Can also be set via PULSON_HOST environment variable [default: 127.0.0.1:3030]
  -h, --help          Print help
```

### Server Management

#### Start Server
```bash
# Basic server startup
pulson --host 127.0.0.1:3030 serve --db-path ~/.local/share/pulson

# With root user capability
pulson --host 127.0.0.1:3030 serve --db-path ~/.local/share/pulson --root-pass SECRET

# With custom thresholds
pulson serve --online-threshold 60 --warning-threshold 600 --stale-threshold 7200

# Enable image storage (disabled by default to save space)
pulson serve --save-images

# Using environment variables
PULSON_HOST=127.0.0.1:3030 pulson serve --db-path ~/.local/share/pulson
```

#### Web Dashboard
Once the server is running, access the dashboard at your configured host address:
- **Local**: http://127.0.0.1:3030
- **Tunnel**: https://your-domain.com

The dashboard provides:
- Real-time device status monitoring
- Interactive data visualizations for all data types
- User authentication and settings management
- Progressive Web App installation options

### Client

The unified `--host` parameter accepts both traditional host:port format and full URLs, making it flexible for different deployment scenarios:

```bash
# Host:port format (local/LAN deployment)
pulson --host 127.0.0.1:3030 device list

# Full URL format (Cloudflare Tunnel/HTTPS)
pulson --host https://sub.domain.com device list

# Using environment variable
export PULSON_HOST=127.0.0.1:3030
pulson device list

# Or with full URL
export PULSON_HOST=https://sub.domain.com
pulson device list
```

#### Register

```bash
pulson --host 127.0.0.1 --port 3030 account register \
  --username myuser --password mypassword [--root-pass ROOT_SECRET]
```

Use `--root-pass` here if you want this new user to have **root** privileges (matching the server’s `--root-pass`).

#### Login

```bash
pulson --host 127.0.0.1:3030 account login \
  --username myuser --password mypassword
```

#### Logout

```bash
pulson --host 127.0.0.1:3030 account logout
```

#### Delete User (root only)

```bash
pulson --host 127.0.0.1:3030 account delete <USERNAME>
```

Only **root** users can delete other accounts.

#### List All Users (root only)

```bash
pulson --host 127.0.0.1:3030 account list
```

Only **root** users can list all registered users and their roles.

#### List Devices

```bash
pulson --host 127.0.0.1:3030 device list
```

#### List Topics for a Device

```bash
pulson --host 127.0.0.1:3030 device list <DEVICE_ID>
```

#### Send Pulse Data

The unified `pulse` command provides a single interface for sending both simple pings and structured data. This replaces the previous separate ping and data commands.

##### Simple Ping (automatic)
When no data is provided, the command automatically sends a ping pulse:
```bash
pulson --host 127.0.0.1:3030 pulse \
  --device-id mydevice --topic "sensors"
```

##### Structured Data with Auto-Detection
The pulse command automatically detects and categorizes your data based on JSON structure:

```bash
# Sensor values (detected as "value" type)
pulson pulse -d mydevice -t /topic/temperature '{"sensor":23.5}'
pulson pulse -d mydevice -t /another/topic/status '{"online":true}'

# Array data (detected as "array" type)  
pulson pulse -d mydevice -t /one/location '{"coordinates":[40.7128, -74.0060, 10]}'

# Event messages (detected as "event" type)
pulson pulse -d mydevice -t /other/event '{"event":"system_startup"}'

# Complex structured data (detected as "value" type)
pulson pulse -d mydevice -t /robo/status '{"status":"operational","uptime":3600,"errors":0}'

# Explicit ping (detected as "ping" type)
pulson pulse -d mydevice -t /just/heartbeat '{"ping":null}'
```

**Data Type Auto-Detection:**
- **Numbers/Booleans**: Categorized as `value` type
- **Arrays**: Categorized as `array` type  
- **Strings**: Categorized as `event` type
- **Objects with null values**: Categorized as `ping` type
- **No data provided**: Automatically sends `{"ping":null}` as `ping` type

You can set the PULSON_HOST environment variable to avoid typing connection parameters every time:

```bash
# Host:port format (local/LAN)
export PULSON_HOST=127.0.0.1:3030
pulson device list
pulson pulse -d foo -t bar                    # Simple ping
pulson pulse -d robot1 -t /top/location '{"coordinates":[lat, lon, alt]}'  # Structured data

# Full URL format (Cloudflare Tunnel/HTTPS)
export PULSON_HOST=https://sub.domain.com
pulson device list
pulson pulse -d foo -t bar                    # Simple ping
pulson pulse -d robot1 -t /top/location '{"coordinates":[lat, lon, alt]}'  # Structured data
```

## Deployment Options

### Local/LAN Deployment

For local development or LAN deployment, use the host:port format:

```bash
# Server
pulson --host 127.0.0.1:3030 serve --db-path ~/.local/share/pulson

# Clients
export PULSON_HOST=127.0.0.1:3030
pulson device list
```

### Internet Deployment (Cloudflare Tunnel)

For secure internet deployment using Cloudflare Tunnel, use the full URL format:

1. **Set up Cloudflare Tunnel** pointing to your local server (127.0.0.1:3030)
2. **Start the server locally**:
   ```bash
   pulson --host 127.0.0.1:3030 serve --db-path ~/.local/share/pulson
   ```

3. **Configure clients to use the tunnel URL**:
   ```bash
   export PULSON_HOST=https://sub.domain.com
   pulson device list
   pulson pulse -d mydevice -t sensors '{"temperature": 23.5}'
   ```

The unified `--host` parameter automatically handles both local and remote deployments seamlessly.

## Configuration & Device Status Thresholds

Pulson supports configurable thresholds for determining device and topic status. These thresholds control when devices are considered **online**, **warning**, **stale**, or **offline**.

### Status Definitions

- **🟢 Online/Active**: Device/topic has sent pings within the online threshold
- **🟡 Warning/Recent**: Device/topic has sent pings within the warning threshold  
- **🔴 Stale**: Topics have sent pings within the stale threshold (topics only)
- **⚫ Offline**: Device/topic has not sent pings beyond the warning/stale threshold

### Configuration Management

#### Show Current Configuration

```bash
pulson config show
```

Display current threshold values and status definitions.

#### Set Thresholds

```bash
# Set individual thresholds (in seconds)
pulson config set --online-threshold 60
pulson config set --warning-threshold 600
pulson config set --stale-threshold 7200

# Set multiple thresholds at once
pulson config set --online-threshold 45 --warning-threshold 300 --stale-threshold 3600
```

#### Configuration File

Configuration is stored in `~/.config/pulson/config.toml`:

```toml
online_threshold_seconds = 30
warning_threshold_seconds = 300
stale_threshold_seconds = 3600
```

#### Dynamic Configuration Updates

When you update configuration using `pulson config set`, the changes take effect immediately:

1. ✅ Configuration file is updated
2. ✅ Running server is automatically notified to reload configuration
3. ✅ Dashboard and API endpoints immediately use new thresholds
4. ✅ **No server restart required!**

#### Server-Side Configuration

You can also override thresholds when starting the server:

```bash
pulson serve --online-threshold 60 --warning-threshold 600 --stale-threshold 7200
```

Or use a custom configuration file:

```bash
pulson serve --config /path/to/config.toml
```

#### Environment Variables

**Connection Configuration:**
```bash
export PULSON_HOST=127.0.0.1:3030         # Host:port format
export PULSON_HOST=https://sub.domain.com  # Full URL format
```

**Threshold Configuration:**
```bash
export PULSON_ONLINE_THRESHOLD=60
export PULSON_WARNING_THRESHOLD=600
export PULSON_STALE_THRESHOLD=7200
```

**Priority order**: CLI arguments > Environment variables > Configuration file > Defaults

## Progressive Web App (PWA) Features

Pulson includes Progressive Web App capabilities, allowing the web dashboard to be installed and used like a native mobile app:

### PWA Features
- **📱 Install as Mobile App**: Install the dashboard directly to your phone's home screen
- **🔄 Offline Support**: Basic offline functionality with cached resources
- **🔔 Push Notifications**: Receive device status alerts (when configured)
- **⚡ Fast Loading**: Service worker caching for improved performance
- **🎨 Native Feel**: App-like experience with proper theming and icons

### Installation
1. **Open the dashboard** in your mobile browser (Chrome/Safari)
2. **Look for the install prompt** or use browser's "Add to Home Screen" option
3. **Install the app** - it will appear on your home screen like a native app

### Testing PWA Features
Access `/pwa-test.html` on your Pulson server to test PWA installation and features:
```bash
# Example: http://127.0.0.1:3030/pwa-test.html
# Or: https://your-tunnel-domain.com/pwa-test.html
```

---


## Installation

### Dependencies

#### Rustup (https://rustup.rs/)

Install 
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Setup
```bash
rustup default stable
```

#### WASM (https://rustwasm.github.io/)
Install
```
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```
Setup
```bash
wasm-pack version
wasm-pack --version
```




### Build

#### Add WASM target

```bash
rustup target add wasm32-unknown-unknown
```

#### Build

```bash
cargo build --release
```


