# QuickShare 🚀

A fast, secure, cross-platform LAN file transfer tool. Send files between devices on your local network without needing the internet.

> Inspired by [LocalSend](https://github.com/localsend/localsend), built with Rust + Tauri.

## Features

- 🚀 **Fast** - Direct peer-to-peer transfer over LAN, no server relay
- 🔒 **Secure** - SHA256 file integrity verification for every transfer
- 📡 **Auto Discovery** - mDNS-based device discovery, zero configuration
- 📱 **Cross-Platform** - macOS (Apple Silicon + Intel), Windows, Linux
- 🎯 **Simple** - Just open and use, no account needed
- 🛠 **CLI + GUI** - Command line interface and graphical interface
- 🌐 **Multi-language** - English / 简体中文
- ⚡ **Real-time Progress** - Live transfer progress via WebSocket
- ✅ **Receive Confirmation** - Accept or reject incoming transfers
- 💾 **Persistent Settings** - Your preferences survive app restarts
- 🔄 **Auto Start** - Option to launch at system startup

## Architecture

```
quickshare/
├── core/          # Core protocol library (Rust)
│   ├── protocol/  # REST API protocol definitions
│   ├── discovery/ # mDNS device discovery
│   ├── transfer/  # File transfer logic with SHA256 verification
│   └── crypto/    # SHA256 hashing & fingerprint generation
├── server/        # HTTP + WebSocket server (Axum)
│   ├── handler/   # REST API handlers with streaming I/O
│   ├── ws/        # WebSocket real-time notifications
│   └── state/     # Persistent application state
├── cli/           # CLI client (Clap)
└── gui/           # GUI app (Tauri + React)
    ├── src-tauri/ # Rust backend
    └── src/       # React frontend
```

## Quick Start

### Download

Download the latest release from the [Releases page](https://github.com/JiGuilin/QuickShare/releases):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `QuickShare_x.x.x_aarch64.dmg` |
| macOS (Intel) | `QuickShare_x.x.x_x64.dmg` |
| Windows | `QuickShare_x.x.x_x64-setup.exe` |
| Linux | `QuickShare_x.x.x_amd64.deb` |

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

**Prerequisites:**
- Rust 1.75+ and Cargo
- Node.js 20+
- Platform-specific dependencies (see Tauri docs)

```bash
# Clone the repository
git clone https://github.com/JiGuilin/QuickShare.git
cd QuickShare

# Build CLI
cargo build --release -p quickshare-cli

# Build GUI (requires Tauri CLI)
cd gui && npm install && npm run tauri build
```

## Protocol

QuickShare uses a simple REST API protocol over HTTP with WebSocket for real-time updates:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/info` | GET | Get device information |
| `/api/devices` | GET | List discovered peer devices |
| `/api/prepare-send` | POST | Prepare to receive files (with accept/reject flow) |
| `/api/accept` | POST | Accept an incoming transfer |
| `/api/reject` | POST | Reject an incoming transfer |
| `/api/send` | POST | Upload file data (multipart with streaming) |
| `/api/cancel` | POST | Cancel a transfer session |
| `/api/settings` | GET/POST | Get or update settings (persisted) |
| `/api/ws` | WebSocket | Real-time notifications, progress, discovery |

### Transfer Flow

1. **Discovery**: Devices find each other via mDNS (`_quickshare._tcp.local.`)
2. **Prepare**: Sender calls `/api/prepare-send` with file metadata (including SHA256)
3. **Accept/Reject**: Receiver confirms or rejects via UI (or auto-accept if enabled)
4. **Upload**: Sender uploads files via multipart, server streams to disk
5. **Progress**: Real-time progress updates via WebSocket (every 200ms)
6. **Verify**: Server verifies file integrity using SHA256 checksum
7. **Complete**: Both parties receive completion notification

## Configuration

Settings are persisted in:
- **macOS**: `~/Library/Application Support/QuickShare/settings.json`
- **Windows**: `%APPDATA%\QuickShare\settings.json`
- **Linux**: `~/.config/QuickShare/settings.json`

## License

MIT
