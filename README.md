# QuickShare 🚀

A fast, secure, cross-platform LAN file transfer tool. Send files between devices on your local network without needing the internet.

> Inspired by [LocalSend](https://github.com/localsend/localsend), built with Rust + Tauri.

## Features

- 🚀 **Fast** - Direct peer-to-peer transfer over LAN, no server relay
- 🔒 **Secure** - HTTPS encryption with TLS/SSL
- 📡 **Auto Discovery** - mDNS-based device discovery, zero configuration
- 📱 **Cross-Platform** - macOS, Windows, Linux, Mobile
- 🎯 **Simple** - Just open and use, no account needed
- 🛠 **CLI + GUI** - Command line interface and graphical interface

## Architecture

```
quickshare/
├── core/          # Core protocol library (Rust)
│   ├── protocol/  # REST API protocol definitions
│   ├── discovery/ # mDNS device discovery
│   ├── transfer/  # File transfer logic
│   └── crypto/    # Encryption & hashing
├── server/        # HTTP + WebSocket server (Axum)
├── cli/           # CLI client (Clap)
└── gui/           # GUI app (Tauri + React)
    ├── src-tauri/ # Rust backend
    └── src/       # React frontend
```

## Quick Start

### CLI Usage

```bash
# Start server (receive mode)
quickshare serve --alias "My Mac" --port 53318

# Send files to another device
quickshare send file1.pdf file2.jpg --target 192.168.1.105

# Discover devices on the network
quickshare discover

# Get info about a remote device
quickshare info --target 192.168.1.105
```

### Build from Source

```bash
# Build CLI
cargo build --release -p quickshare-cli

# Build GUI (requires Tauri CLI)
cd gui && npm install && npm run tauri build
```

## Protocol

QuickShare uses a simple REST API protocol over HTTPS:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/info` | GET | Get device information |
| `/api/prepare-send` | POST | Prepare to receive files |
| `/api/send` | POST | Upload file data (multipart) |
| `/api/cancel` | POST | Cancel a transfer session |
| `/api/ws` | WebSocket | Real-time notifications & discovery |

## License

MIT
