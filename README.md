# QuickShare 🚀

A fast, secure, cross-platform LAN file transfer tool. Send files between devices on your local network without needing the internet.

> Inspired by [LocalSend](https://github.com/localsend/localsend), built with Rust + Tauri.

[中文文档](./README_zh.md)

## ✨ Features

- 🚀 **Fast** - Direct peer-to-peer transfer over LAN, no server relay
- 📦 **Large File Support** - Chunked upload (4MB chunks) for files of any size, no memory overflow
- 🔒 **Secure** - SHA256 file integrity verification for every transfer
- 📡 **Auto Discovery** - UDP multicast + mDNS dual discovery, zero configuration
- 📱 **Cross-Platform** - macOS (Apple Silicon + Intel), Windows, Linux
- 🎯 **Simple** - Just open and use, no account needed
- 🛠 **CLI + GUI** - Command line interface and graphical interface
- 🌐 **Multi-language** - English / 简体中文
- ⚡ **Real-time Progress** - Live transfer progress with speed display via WebSocket
- ✅ **Receive Confirmation** - Accept or reject incoming transfers
- 🤖 **Auto-accept** - Toggle auto-accept mode to receive files without confirmation
- 💾 **Persistent Settings** - Your preferences survive app restarts
- 🎲 **Random Alias** - Auto-generated fun device names like "Cute Mango" (inspired by LocalSend)
- 🔄 **Auto Start** - Option to launch at system startup
- 🔍 **Network Scan** - Trigger on-demand device discovery with one click
- 🔗 **HTTP Polling** - Reliable cross-device session status checking (no missed WebSocket messages)
- 🖥️ **This Device Info** - View your own device alias, IP and port in the receive tab
- 📂 **Open Directory** - One-click to open the download folder from transfer history
- 📋 **Send History** - View all completed send transfers with file details
- 📋 **Receive History** - View all completed receive transfers with expandable file list
- 🗑️ **Remove Files** - Remove individual files from the send queue before sending
- 🖱️ **Drag & Drop** - Drag files directly to the send area (optimized for large files)
- 🎨 **Smart UI** - Upload icon and hint text auto-hide when files are selected

## 🆕 What's New in v0.3.0

- 🖥️ **This Device Card** - Receive tab now shows your device alias, IP and port (LocalSend style)
- 🤖 **Auto-accept Toggle** - Quick toggle auto-accept mode right from the receive tab
- 📂 **Open Download Folder** - Click to open the download directory from receive history
- 📋 **Send History** - New send history panel with clock icon button, view all completed sends
- 📋 **Receive History** - Improved receive history with expandable multi-file details
- 🗑️ **Remove Files** - Remove individual files from the send queue with ✕ button
- 🖱️ **Optimized Drag & Drop** - Large file drag & drop is now instant (only reads metadata on drop, reads content on send)
- 🎨 **Smart File Area** - Upload icon and hint text auto-hide when files are selected, shows "add more" hint instead
- 🔧 **Windows Fixes** - Fixed Chinese alias display, hidden console window, open directory works on Windows
- 🔧 **WebSocket Stability** - Fixed React StrictMode race condition causing duplicate event listeners

## 🏗 Architecture

```
quickshare/
├── core/          # Core protocol library (Rust)
│   ├── alias/     # Random alias generator (adjective + fruit)
│   ├── protocol/  # REST API protocol definitions
│   ├── discovery/ # UDP multicast + mDNS device discovery
│   ├── transfer/  # File transfer logic with SHA256 verification
│   └── crypto/    # SHA256 hashing & fingerprint generation
├── server/        # HTTP + WebSocket server (Axum)
│   ├── handler/   # REST API handlers with streaming I/O + chunked upload
│   ├── ws/        # WebSocket real-time notifications
│   └── state/     # Persistent application state
├── cli/           # CLI client (Clap)
└── gui/           # GUI app (Tauri + React)
    ├── src-tauri/ # Rust backend
    └── src/       # React frontend
```

## 🚀 Quick Start

### Download

Download the latest release from the [Releases page](https://github.com/JiGuilin/QuickShare/releases):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `QuickShare_0.3.0_aarch64.dmg` |
| macOS (Intel) | `QuickShare_0.3.0_x64.dmg` |
| Windows | `QuickShare_0.3.0_x64-setup.exe` |
| Linux | `QuickShare_0.3.0_amd64.deb` |

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
- Platform-specific dependencies (see [Tauri docs](https://tauri.app/start/prerequisites/))

```bash
# Clone the repository
git clone https://github.com/JiGuilin/QuickShare.git
cd QuickShare

# Build CLI
cargo build --release -p quickshare-cli

# Build GUI (requires Tauri CLI)
cd gui && npm install && npm run tauri build
```

## 📡 Protocol

QuickShare uses a simple REST API protocol over HTTP with WebSocket for real-time updates:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/info` | GET | Get device information |
| `/api/devices` | GET | List discovered peer devices |
| `/api/prepare-send` | POST | Prepare to receive files (with accept/reject flow) |
| `/api/accept` | POST | Accept an incoming transfer |
| `/api/reject` | POST | Reject an incoming transfer |
| `/api/send` | POST | Upload file data (multipart with streaming, for small files <8MB) |
| `/api/upload-chunk` | POST | Upload a file chunk (for large files ≥8MB) |
| `/api/cancel` | POST | Cancel a transfer session |
| `/api/session-status/{id}` | GET | Query session status (for sender polling) |
| `/api/settings` | GET/POST | Get or update settings (persisted) |
| `/api/random-alias` | GET | Generate a random device alias (`?locale=en|zh`) |
| `/api/scan` | POST | Trigger network scan (sends multicast announcement) |
| `/api/ws` | WebSocket | Real-time notifications, progress, discovery |

### Device Discovery

QuickShare uses a **dual discovery mechanism** for maximum compatibility:

1. **UDP Multicast** (primary) - Sends JSON announcements to multicast group `224.0.0.167:53318`, compatible with LocalSend's discovery protocol
2. **mDNS** (supplementary) - Registers `_quickshare._tcp.local.` service for networks that support Bonjour/Avahi

All discovery protocols use the **SHA256 fingerprint** as the unique device ID, ensuring the same device is never listed twice regardless of which protocol found it.

When a device starts, it:
1. Binds a UDP socket and joins the multicast group on all local interfaces
2. Registers an mDNS service on the network
3. Sends an initial multicast announcement
4. Listens for announcements from other devices and responds automatically

When you click **Scan** in the GUI, a multicast announcement is sent, and all listening QuickShare devices respond with their info.

### Large File Transfer (Chunked Upload)

QuickShare supports transferring files of **any size** using a chunked upload mechanism:

- **Small files (< 8MB)**: Uploaded in a single multipart request via `/api/send`
- **Large files (≥ 8MB)**: Split into **4MB chunks** and uploaded sequentially via `/api/upload-chunk`

**How it works:**

1. The frontend reads the file using `File.slice()` — only one chunk is loaded into memory at a time
2. Each chunk is sent as a raw binary POST request with metadata in query params:
   - `session_id` — Transfer session ID
   - `file_name` — Original file name
   - `chunk_index` — 0-based chunk index
   - `total_chunks` — Total number of chunks for this file
   - `is_file_done` — `true` when the last chunk of a file is uploaded (triggers file assembly)
   - `is_session_done` — `true` when the last chunk of the last file is uploaded (marks session complete)
3. The server writes each chunk to a temporary directory
4. When `is_file_done=true`, the server assembles all chunks into the final file and cleans up temp files
5. Real-time progress with **transfer speed** is shown in the UI

This approach ensures **constant memory usage** regardless of file size — a 10GB file uses the same memory as a 10MB file.

### Transfer Flow

1. **Discovery**: Devices find each other via UDP multicast and/or mDNS
2. **Prepare**: Sender calls `/api/prepare-send` with file metadata (including SHA256)
3. **Accept/Reject**: Receiver confirms or rejects via UI (or auto-accept if enabled)
4. **Wait**: Sender polls `/api/session-status/{id}` until the receiver accepts (more reliable than cross-device WebSocket)
5. **Upload**: Small files use multipart; large files use chunked upload
6. **Progress**: Real-time progress with speed display via WebSocket (updated per chunk)
7. **Verify**: Server verifies file integrity using SHA256 checksum
8. **Complete**: Both parties receive completion notification

### Random Alias

On first launch, QuickShare automatically generates a fun device name using an **adjective + fruit** combination, just like LocalSend:

- 🇺🇸 English: `"Cute Mango"`, `"Fast Lemon"`, `"Smart Pineapple"` … (988 combinations)
- 🇨🇳 Chinese: `"可爱的芒果"`, `"快速的柠檬"`, `"聪明的菠萝"` … (988 combinations)

You can also:
- Click the **⚡ Random Alias** button in Settings to regenerate
- Click the **🖥️ System Name** button to use your computer's hostname
- Manually type any name you like

## ⚙️ Configuration

Settings are persisted in:
- **macOS**: `~/Library/Application Support/QuickShare/settings.json`
- **Windows**: `%APPDATA%\QuickShare\settings.json`
- **Linux**: `~/.config/QuickShare/settings.json`

Example `settings.json`:
```json
{
  "alias": "Cute Mango",
  "download_dir": "/Users/john/Downloads/QuickShare",
  "auto_accept": false,
  "fingerprint": "a1b2c3d4e5f67890abcdef1234567890"
}
```

## 🔧 Troubleshooting

### Devices not discovered

If you cannot see other devices on the network:
1. Make sure all devices are on the **same Wi-Fi/network**
2. Check that **UDP port 53318** is not blocked by your firewall
3. Try clicking the **Scan** button to trigger a multicast announcement
4. On macOS, ensure the app has **Local Network** permission in System Settings

### Firewall settings

QuickShare needs the following ports open:
- **TCP 53318** - HTTP server for file transfer
- **UDP 53318** - Multicast discovery

### Large file transfer fails

If large file transfers fail or get stuck:
1. Check that the receiving device has enough **disk space**
2. Ensure a **stable Wi-Fi** connection — avoid networks with heavy interference
3. The temp directory (`/tmp/quickshare-chunks/`) is cleaned up automatically after each session
4. If a transfer is interrupted, stale temp files may remain — you can safely delete them manually

### Duplicate devices in the list

QuickShare uses a unified SHA256 fingerprint as the device ID across all discovery protocols. If you still see duplicates:
1. Make sure both devices are running the **same version** of QuickShare
2. Try clicking **Scan** again to refresh the device list

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

MIT
