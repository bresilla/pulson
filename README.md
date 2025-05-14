

<img align="right" width="26%" src="./book/src/images/logo.png">

pulson
===


realtime system/robot monitoring and tracing


## Usage

```bash
pulson --help

realtime system/robot monitoring and tracing

Usage: pulson [OPTIONS] <COMMAND>

Commands:
  serve    Run the HTTP server
  list     Query the running server for all tracked devices (or topics for one)
  ping     Send a ping (POST /ping) for a given device_id and topic
  account  User account management (register, login, logout)
  help     Print this message or the help of the given subcommand(s)

Options:
  -H, --host <HOST>  Address to bind (serve) or connect to (client) [default: 127.0.0.1]
  -p, --port <PORT>  Port to bind or connect to [default: 3030]
  -h, --help         Print help

```

### Server

```bash
pulson --host 127.0.0.1 --port 3030 serve --db-path ~/.local/share/pulson
```

### Client

#### Register

```bash
pulson --host 127.0.0.1 --port 3030 account register --username myuser --password mypassword
```

#### Login

```bash
pulson --host 127.0.0.1 --port 3030 account login --username myuser --password mypassword
```

#### List

```bash
pulson --host 127.0.0.1 --port 3030 list
```

#### Ping

```bash
pulson --host 127.0.0.1 --port 3030 ping --device-id mydevice --topic "my/topic/pulse"
```
