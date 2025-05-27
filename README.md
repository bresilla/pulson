<img align="right" width="26%" src="./book/src/images/logo.png">

# pulson

realtime system/robot monitoring and tracing

## Usage

```bash
pulson --help

realtime system/robot monitoring and tracing

Usage: pulson [OPTIONS] <COMMAND>

Commands:
  serve         Run the HTTP server
  device        Device management (list, delete)
  pulse         Send a unified pulse with optional structured data payload
  account       User account management (register, login, logout, delete, list)
  config        Configuration management (show, set thresholds)
  help          Print this message or the help of the given subcommand(s)

Options:
  -H, --host <HOST>        Address to bind (serve) or connect to (client) [default: 127.0.0.1]
  -p, --port <PORT>        Port to bind or connect to [default: 3030]
  -h, --help               Print help
```

### Server

```bash
pulson --host 127.0.0.1 --port 3030 serve \
  --db-path ~/.local/share/pulson [--root-pass <ROOT_SECRET>]
```

If you supply the correct `--root-pass` when starting the server, the first user who registers with that secret becomes a **root** user.

The server provides a web dashboard at `http://127.0.0.1:3030` that displays real-time device status using the configured thresholds.

### Client

#### Register

```bash
pulson --host 127.0.0.1 --port 3030 account register \
  --username myuser --password mypassword [--root-pass ROOT_SECRET]
```

Use `--root-pass` here if you want this new user to have **root** privileges (matching the server’s `--root-pass`).

#### Login

```bash
pulson --host 127.0.0.1 --port 3030 account login \
  --username myuser --password mypassword
```

#### Logout

```bash
pulson --host 127.0.0.1 --port 3030 account logout
```

#### Delete User (root only)

```bash
pulson --host 127.0.0.1 --port 3030 account delete <USERNAME>
```

Only **root** users can delete other accounts.

#### List All Users (root only)

```bash
pulson --host 127.0.0.1 --port 3030 account list
```

Only **root** users can list all registered users and their roles.

#### List Devices

```bash
pulson --host 127.0.0.1 --port 3030 list
```

#### List Topics for a Device

```bash
pulson --host 127.0.0.1 --port 3030 device list <DEVICE_ID>
```

#### Send Pulse Data

The unified `pulse` command provides a single interface for sending both simple pings and structured data. This replaces the previous separate ping and data commands.

##### Simple Ping (automatic)
When no data is provided, the command automatically sends a ping pulse:
```bash
pulson --host 127.0.0.1 --port 3030 pulse \
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

You can set environment variables PULSON_IP and PULSON_PORT to avoid typing --host/--port every time:

```bash
export PULSON_IP=127.0.0.1
export PULSON_PORT=3030

pulson device list
pulson pulse -d foo -t bar                    # Simple ping
pulson pulse -d robot1 -t /top/location '{"coordinates":[lat, lon, alt]}'  # Structured data
```

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

Thresholds can be set via environment variables:

```bash
export PULSON_ONLINE_THRESHOLD=60
export PULSON_WARNING_THRESHOLD=600
export PULSON_STALE_THRESHOLD=7200
```

**Priority order**: CLI arguments > Environment variables > Configuration file > Defaults

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


