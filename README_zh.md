# QuickShare 🚀

快速、安全、跨平台的局域网文件传输工具。在局域网内设备之间发送文件，无需互联网连接。

> 灵感来自 [LocalSend](https://github.com/localsend/localsend)，使用 Rust + Tauri 构建。

[English](./README.md)

## ✨ 功能特性

- 🚀 **快速** - 局域网直连传输，无需服务器中转
- 🔒 **安全** - 每次传输都进行 SHA256 文件完整性校验
- 📡 **自动发现** - UDP 多播 + mDNS 双重发现机制，零配置
- 📱 **跨平台** - macOS (Apple Silicon + Intel)、Windows、Linux
- 🎯 **简单** - 打开即用，无需注册账号
- 🛠 **CLI + GUI** - 命令行界面和图形界面双模式
- 🌐 **多语言** - English / 简体中文
- ⚡ **实时进度** - 通过 WebSocket 实时显示传输进度
- ✅ **接收确认** - 可接受或拒绝传入的文件传输
- 💾 **设置持久化** - 您的偏好设置在重启后保留
- 🎲 **随机别名** - 自动生成有趣的设备名，如「可爱的芒果」（灵感来自 LocalSend）
- 🔄 **开机自启** - 可选开机自动启动
- 🔍 **网络扫描** - 一键触发设备发现

## 🏗 项目架构

```
quickshare/
├── core/          # 核心协议库 (Rust)
│   ├── alias/     # 随机别名生成器（形容词 + 水果）
│   ├── protocol/  # REST API 协议定义
│   ├── discovery/ # UDP 多播 + mDNS 设备发现
│   ├── transfer/  # 文件传输逻辑（含 SHA256 校验）
│   └── crypto/    # SHA256 哈希与指纹生成
├── server/        # HTTP + WebSocket 服务器 (Axum)
│   ├── handler/   # REST API 处理器（流式 I/O）
│   ├── ws/        # WebSocket 实时通知
│   └── state/     # 持久化应用状态
├── cli/           # 命令行客户端 (Clap)
└── gui/           # 图形界面应用 (Tauri + React)
    ├── src-tauri/ # Rust 后端
    └── src/       # React 前端
```

## 🚀 快速开始

### 下载安装

从 [Releases 页面](https://github.com/JiGuilin/QuickShare/releases) 下载最新版本：

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `QuickShare_x.x.x_aarch64.dmg` |
| macOS (Intel) | `QuickShare_x.x.x_x64.dmg` |
| Windows | `QuickShare_x.x.x_x64-setup.exe` |
| Linux | `QuickShare_x.x.x_amd64.deb` |

### 命令行使用

```bash
# 启动服务器（接收模式）
quickshare serve --alias "我的电脑" --port 53318

# 发送文件到其他设备
quickshare send file1.pdf file2.jpg --target 192.168.1.105

# 发现局域网内的设备
quickshare discover

# 获取远程设备信息
quickshare info --target 192.168.1.105
```

### 从源码构建

**前置条件：**
- Rust 1.75+ 和 Cargo
- Node.js 20+
- 平台相关依赖（参见 [Tauri 文档](https://tauri.app/start/prerequisites/)）

```bash
# 克隆仓库
git clone https://github.com/JiGuilin/QuickShare.git
cd QuickShare

# 构建 CLI
cargo build --release -p quickshare-cli

# 构建 GUI（需要 Tauri CLI）
cd gui && npm install && npm run tauri build
```

## 📡 通信协议

QuickShare 使用基于 HTTP 的 REST API 协议，配合 WebSocket 实现实时更新：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/info` | GET | 获取设备信息 |
| `/api/devices` | GET | 列出已发现的设备 |
| `/api/prepare-send` | POST | 准备接收文件（含确认/拒绝流程） |
| `/api/accept` | POST | 接受传入的传输 |
| `/api/reject` | POST | 拒绝传入的传输 |
| `/api/send` | POST | 上传文件数据（multipart 流式传输） |
| `/api/cancel` | POST | 取消传输会话 |
| `/api/settings` | GET/POST | 获取或更新设置（持久化） |
| `/api/random-alias` | GET | 生成随机设备别名（`?locale=en|zh`） |
| `/api/scan` | POST | 触发网络扫描（发送多播公告） |
| `/api/ws` | WebSocket | 实时通知、进度、设备发现 |

### 设备发现

QuickShare 使用**双重发现机制**，确保最大兼容性：

1. **UDP 多播**（主要方式）- 向多播组 `224.0.0.167:53318` 发送 JSON 公告，与 LocalSend 的发现协议兼容
2. **mDNS**（辅助方式）- 注册 `_quickshare._tcp.local.` 服务，适用于支持 Bonjour/Avahi 的网络

设备启动时会：
1. 绑定 UDP socket 并在所有本地接口加入多播组
2. 在网络上注册 mDNS 服务
3. 发送初始多播公告
4. 监听其他设备的公告并自动回复

在 GUI 中点击**扫描**按钮时，会发送一个多播公告，所有正在监听的 QuickShare 设备都会回复自己的信息。

### 传输流程

1. **发现**：设备通过 UDP 多播和/或 mDNS 互相发现
2. **准备**：发送方调用 `/api/prepare-send` 传入文件元数据（含 SHA256）
3. **确认/拒绝**：接收方通过界面确认或拒绝（或启用自动接收）
4. **上传**：发送方通过 multipart 上传文件，服务器流式写入磁盘
5. **进度**：通过 WebSocket 实时推送传输进度（每 200ms）
6. **校验**：服务器使用 SHA256 校验文件完整性
7. **完成**：双方收到传输完成通知

### 随机别名

首次启动时，QuickShare 会自动使用**形容词 + 水果**组合生成一个有趣的设备名，与 LocalSend 的命名方式一致：

- 🇺🇸 英文：`"Cute Mango"`、`"Fast Lemon"`、`"Smart Pineapple"` ……（988 种组合）
- 🇨🇳 中文：`"可爱的芒果"`、`"快速的柠檬"`、`"聪明的菠萝"` ……（988 种组合）

您还可以：
- 点击设置中的 **⚡ 随机别名** 按钮重新生成
- 点击 **🖥️ 系统名称** 按钮使用电脑主机名
- 手动输入您喜欢的任意名称

## ⚙️ 配置

设置持久化存储位置：
- **macOS**：`~/Library/Application Support/QuickShare/settings.json`
- **Windows**：`%APPDATA%\QuickShare\settings.json`
- **Linux**：`~/.config/QuickShare/settings.json`

配置示例 `settings.json`：
```json
{
  "alias": "可爱的芒果",
  "download_dir": "/Users/john/Downloads/QuickShare",
  "auto_accept": false,
  "fingerprint": "a1b2c3d4e5f67890abcdef1234567890"
}
```

## 🔧 常见问题

### 无法发现设备

如果无法看到网络中的其他设备：
1. 确保所有设备连接在**同一 Wi-Fi/网络**下
2. 检查**UDP 端口 53318** 是否被防火墙阻止
3. 尝试点击**扫描**按钮触发多播公告
4. 在 macOS 上，确保应用已在系统设置中获得**本地网络**权限

### 防火墙设置

QuickShare 需要开放以下端口：
- **TCP 53318** - HTTP 文件传输服务器
- **UDP 53318** - 多播设备发现

## 🤝 参与贡献

欢迎贡献！请随时提交 Pull Request。

1. Fork 本仓库
2. 创建功能分支（`git checkout -b feature/amazing-feature`）
3. 提交更改（`git commit -m 'Add amazing feature'`）
4. 推送到分支（`git push origin feature/amazing-feature`）
5. 发起 Pull Request

## 📄 许可证

MIT
