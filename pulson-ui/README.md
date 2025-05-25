# Pulson UI

A modern web-based dashboard for monitoring and managing Pulson devices and topics in real-time.

## Features

- **Real-time Device Monitoring**: View all connected devices with their last seen timestamps
- **Topic Management**: Browse topics for each device with activity indicators
- **Live Status Indicators**: Color-coded status showing device/topic activity levels
- **Auto-refresh**: Automatic data updates every 5 seconds (toggleable)
- **Responsive Design**: Works on desktop, tablet, and mobile devices
- **Modern UI**: Clean, professional interface with intuitive navigation

## Technology Stack

- **Frontend**: Yew (Rust WASM framework)
- **Styling**: Custom CSS with modern design principles
- **State Management**: Yew hooks and local storage
- **HTTP Client**: gloo-net for API communication
- **Build Tool**: wasm-pack

## Development Setup

### Prerequisites

1. Install Rust and wasm-pack:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Building the UI

The UI is automatically built when you build the main Pulson project. From the project root:

```bash
# Build the entire project (including UI)
cargo build --release

# Or for development builds
cargo build
```

This will:
1. Automatically trigger the UI build via `build.rs`
2. Compile the Rust UI code to WASM using wasm-pack
3. Generate JavaScript bindings
4. Copy static files to the distribution directory
5. Embed the UI bundle in the main Pulson binary

The build process is integrated into Pulson's `build.rs` script, so any changes to the UI source files will automatically trigger a rebuild.

**Note**: The first build may take a few minutes as it downloads and compiles WASM dependencies.

### Running the Development Server

The UI is served by the main Pulson server. From the project root:

```bash
# Start the server (UI is always available)
cargo run -- serve --host 127.0.0.1 --port 3030

# Or for development with auto-rebuild on changes
cargo run -- serve --host 127.0.0.1 --port 3030
```

The UI is automatically embedded and served at the root URL. Visit http://127.0.0.1:3030 in your browser.

**Note**: The `--webui` flag is not required - the UI is always available when the server runs.

## Project Structure

```
pulson-ui/
├── src/
│   ├── components/
│   │   ├── dashboard.rs    # Main dashboard component
│   │   └── mod.rs         # Component exports
│   └── lib.rs             # Entry point, routing, login
├── static/
│   ├── index.html         # HTML template
│   └── style.css          # Stylesheet
├── ui/dist/               # Build output (generated)
└── Cargo.toml            # Dependencies
```

## API Integration

The UI communicates with the Pulson server via REST API:

- `POST /account/login` - User authentication
- `GET /devices` - List all devices with last seen timestamps
- `GET /devices/{id}` - Get topics for a specific device
- `POST /ping` - Send a ping for a device/topic

Authentication is handled via Bearer tokens stored in browser localStorage.

## UI Components

### Dashboard
- **Device Panel**: Lists all devices with status indicators
- **Topic Panel**: Shows topics for selected device
- **Statistics Panel**: Displays counts and system info
- **Navigation**: Logout, refresh, and auto-refresh controls

### Status Indicators
- 🟢 **Online/Active**: Activity within 30 seconds
- 🟡 **Warning/Recent**: Activity within 5 minutes
- 🟠 **Stale**: Activity within 1 hour
- 🔴 **Offline/Inactive**: No recent activity

## Styling

The UI uses a modern design system with:
- Responsive grid layout
- Color-coded status indicators
- Smooth animations and transitions
- Professional typography
- Mobile-first responsive design

## Development Notes

- The UI is built automatically when building the main Pulson project
- UI assets are embedded in the binary using `rust-embed`
- The UI automatically refreshes data every 5 seconds when auto-refresh is enabled
- User authentication state is persisted in localStorage
- The dashboard shows real-time relative timestamps (e.g., "2m ago")
- Device and topic lists are scrollable for large datasets
- Error states are handled gracefully with user-friendly messages
- Changes to UI source files trigger automatic rebuilds via `build.rs`

## Future Enhancements

- [ ] Ping form for sending test pings
- [ ] Real-time WebSocket updates
- [ ] Device grouping and filtering
- [ ] Historical data visualization
- [ ] User management interface
- [ ] Export functionality
- [ ] Dark mode support
- [ ] Performance metrics dashboard