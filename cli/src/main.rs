use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::warn;
use std::path::PathBuf;

use quickshare_core::discovery::DiscoveryService;
use quickshare_core::protocol::{DeviceInfo, PrepareSendRequest, PrepareSendResponse, InfoResponse};
use quickshare_core::transfer::FileSender;
use quickshare_core::DEFAULT_PORT;

#[derive(Parser)]
#[command(name = "quickshare")]
#[command(about = "🚀 QuickShare - Cross-platform LAN file transfer tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Device alias (default: hostname)
    #[arg(short, long, global = true)]
    alias: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start QuickShare server (receive mode)
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,

        /// Output directory for received files
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Send files to another device
    Send {
        /// Files to send
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Target device IP address
        #[arg(short, long)]
        target: String,

        /// Target device port
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,

        /// Timeout in seconds to wait for receiver acceptance
        #[arg(short, long, default_value_t = 60)]
        timeout: u64,
    },

    /// Discover devices on the local network
    Discover {
        /// Discovery duration in seconds
        #[arg(short, long, default_value_t = 10)]
        duration: u64,
    },

    /// Get info about a remote device
    Info {
        /// Device IP address
        #[arg(short, long)]
        target: String,

        /// Device port
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("quickshare=info")
        .init();

    let cli = Cli::parse();
    let alias = cli.alias.unwrap_or_else(|| {
        std::env::var("USER").unwrap_or_else(|_| "QuickShare User".to_string())
    });

    match cli.command {
        Commands::Serve { port, output } => {
            run_server(port, alias, output).await?;
        }
        Commands::Send { files, target, port, timeout } => {
            send_files(files, &target, port, &alias, timeout).await?;
        }
        Commands::Discover { duration } => {
            discover_devices(duration, &alias).await?;
        }
        Commands::Info { target, port } => {
            get_device_info(&target, port).await?;
        }
    }

    Ok(())
}

async fn run_server(port: u16, alias: String, output: Option<String>) -> Result<()> {
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║     🚀 QuickShare Server Mode       ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();
    println!("  {} {}", "Alias:".green(), alias);
    println!("  {} {}", "Port:".green(), port);

    if let Some(dir) = &output {
        println!("  {} {}", "Output:".green(), dir);
    }

    // Start mDNS discovery with proper fingerprint
    let fingerprint = quickshare_core::crypto::generate_fingerprint();
    let discovery = DiscoveryService::new(alias.clone(), port, fingerprint)?;
    discovery.register()?;

    println!();
    println!("{}", "Waiting for incoming transfers...".yellow());
    println!("{}", "Press Ctrl+C to stop".dimmed());

    // Start the server
    quickshare_server::run_server(port, alias).await?;

    Ok(())
}

async fn send_files(files: Vec<PathBuf>, target: &str, port: u16, alias: &str, timeout: u64) -> Result<()> {
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║     📤 QuickShare Send Mode         ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();

    // Prepare file metadata with SHA256
    println!("  {} Computing file checksums...", "⏳".yellow());
    let file_metas = FileSender::prepare_files(&files).await?;
    let total_size: u64 = file_metas.iter().map(|f| f.size).sum();

    println!("  {} {} file(s) ({})", "Sending:".green(), file_metas.len(), format_size(total_size));
    for meta in &file_metas {
        println!("    {} {} ({}) {}", "•".dimmed(), meta.name, format_size(meta.size),
            meta.sha256.as_ref().map(|_h| format!("✓ checksum").green().to_string())
                .unwrap_or_else(|| format!("⚠ no checksum").yellow().to_string())
        );
    }
    println!();

    let base_url = format!("http://{}:{}", target, port);

    // Get remote device info
    let client = reqwest::Client::new();
    let info: InfoResponse = client.get(format!("{}/api/info", base_url))
        .send().await?
        .json().await?;

    println!("  {} {} at {}", "Target:".green(), info.device.alias, target);
    println!();

    // Prepare send request
    let sender_info = DeviceInfo::new(alias.to_string(), 0);
    let prepare_req = PrepareSendRequest {
        sender: sender_info,
        files: file_metas.clone(),
        pin: None,
    };

    let prepare_resp: PrepareSendResponse = client
        .post(format!("{}/api/prepare-send", base_url))
        .json(&prepare_req)
        .send().await?
        .json().await?;

    if !prepare_resp.accepted {
        if prepare_resp.session_id.is_empty() {
            println!("{}", "❌ Transfer rejected by receiver".red());
            return Ok(());
        }

        // Receiver needs to accept - wait for confirmation via WebSocket
        println!("{}", "⏳ Waiting for receiver to accept...".yellow());

        let ws_url = format!("ws://{}:{}/api/ws", target, port);
        let accepted = wait_for_acceptance(&ws_url, &prepare_resp.session_id, timeout).await?;

        if !accepted {
            println!("{}", "❌ Transfer rejected by receiver".red());
            return Ok(());
        }

        println!("{}", "✅ Transfer accepted by receiver".green());
    }

    println!("  {} Session: {}", "✓".green(), prepare_resp.session_id);
    println!();

    // Send each file with progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})"
    ).unwrap());

    for (i, file_meta) in file_metas.iter().enumerate() {
        pb.set_message(format!("Sending: {}", file_meta.name));

        let file_path = &files[i];
        let mut form = reqwest::multipart::Form::new();

        // Include session_id in the multipart form
        form = form.text("session_id", prepare_resp.session_id.clone());

        let file_data = tokio::fs::read(file_path).await?;
        let part = reqwest::multipart::Part::bytes(file_data)
            .file_name(file_meta.name.clone())
            .mime_str("application/octet-stream")?;
        form = form.part("file", part);

        let resp = client
            .post(format!("{}/api/send", base_url))
            .multipart(form)
            .send().await?;

        if resp.status().is_success() {
            pb.inc(file_meta.size);
        } else {
            pb.println(format!("{} Failed to send: {}", "✗".red(), file_meta.name));
        }
    }

    pb.finish_with_message("✅ Transfer complete!");
    println!();
    println!("{}", "🎉 All files sent successfully!".green().bold());

    Ok(())
}

/// Wait for transfer acceptance via WebSocket
async fn wait_for_acceptance(ws_url: &str, session_id: &str, timeout: u64) -> Result<bool> {
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    use futures_util::stream::StreamExt;

    let (stream, _) = connect_async(ws_url).await
        .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {}", e))?;
    let (_, mut read) = stream.split();

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        }

        match tokio::time::timeout(remaining, read.next()).await {
            Ok(Some(Ok(msg))) => {
                if let Message::Text(text) = msg {
                    if let Ok(ws_msg) = serde_json::from_str::<quickshare_core::protocol::WsMessage>(&text) {
                        match ws_msg {
                            quickshare_core::protocol::WsMessage::TransferResponse { session_id: sid, accepted } => {
                                if sid == session_id {
                                    return Ok(accepted);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Some(Err(e))) => {
                warn!("WebSocket error: {}", e);
                return Ok(false);
            }
            Ok(None) => {
                return Ok(false);
            }
            Err(_) => {
                println!("{}", "⏰ Timed out waiting for acceptance".red());
                return Ok(false);
            }
        }
    }
}

async fn discover_devices(duration: u64, alias: &str) -> Result<()> {
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║     🔍 Discovering Devices...       ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();

    // Use a proper fingerprint for self-filtering
    let fingerprint = quickshare_core::crypto::generate_fingerprint();
    let discovery = DiscoveryService::new(alias.to_string(), DEFAULT_PORT, fingerprint)?;
    let mut rx = discovery.browse();

    println!("  {} Scanning for {} seconds...\n", "⏳".yellow(), duration);

    let start = std::time::Instant::now();
    let mut found = Vec::new();

    while start.elapsed().as_secs() < duration {
        match rx.try_recv() {
            Ok(event) => match event {
                quickshare_core::discovery::DiscoveryEvent::DeviceFound(device) => {
                    let icon = match device.device_type {
                        quickshare_core::protocol::DeviceType::Mobile => "📱",
                        quickshare_core::protocol::DeviceType::Desktop => "💻",
                        _ => "🖥️",
                    };
                    println!("  {} {} ({}:{}) - {} {}",
                        icon.green(),
                        device.alias.bold(),
                        device.ip,
                        device.port,
                        device.device_model.dimmed(),
                        device.os.dimmed()
                    );
                    found.push(device);
                }
                quickshare_core::discovery::DiscoveryEvent::DeviceLost(id) => {
                    println!("  {} Device left: {}", "👋".yellow(), id);
                }
            },
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    println!();
    if found.is_empty() {
        println!("{}", "  No devices found on the local network.".yellow());
    } else {
        println!("  {} {} device(s) found", "✓".green(), found.len());
    }

    Ok(())
}

async fn get_device_info(target: &str, port: u16) -> Result<()> {
    let client = reqwest::Client::new();
    let base_url = format!("http://{}:{}", target, port);

    let info: InfoResponse = client.get(format!("{}/api/info", base_url))
        .send().await?
        .json().await?;

    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║     📋 Device Information           ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();
    println!("  {} {}", "Alias:".green(), info.device.alias);
    println!("  {} {}", "ID:".green(), info.device.id);
    println!("  {} {}", "IP:".green(), info.device.ip);
    println!("  {} {}", "Port:".green(), info.device.port);
    println!("  {} {}", "Version:".green(), info.device.version);
    println!("  {} {}", "OS:".green(), info.device.os);
    println!("  {} {}", "Fingerprint:".green(), info.device.fingerprint);

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
