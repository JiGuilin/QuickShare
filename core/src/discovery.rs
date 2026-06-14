use anyhow::Result;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::protocol::{DeviceInfo, DeviceType};
use crate::SERVICE_TYPE;

// ─── UDP Multicast Constants (LocalSend-compatible) ───────────
/// Default multicast group - 224.0.0.0/24 is the most compatible range
pub const MULTICAST_GROUP: &str = "224.0.0.167";

/// Discovery event sent through the channel
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    DeviceFound(DeviceInfo),
    DeviceLost(String),
}

/// Information announced over UDP multicast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MulticastAnnouncement {
    pub alias: String,
    pub version: String,
    pub fingerprint: String,
    pub port: u16,
    pub os: String,
    pub device_type: String,
    pub announcement: bool,
}

impl MulticastAnnouncement {
    /// Convert to UDP bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Parse from UDP bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}

// ─── UDP Multicast Discovery ──────────────────────────────────

/// UDP multicast discovery service (LocalSend-compatible)
pub struct MulticastDiscovery {
    alias: std::sync::Mutex<String>,
    port: u16,
    fingerprint: String,
    multicast_addr: Ipv4Addr,
    multicast_port: u16,
}

impl MulticastDiscovery {
    pub fn new(alias: String, port: u16, fingerprint: String) -> Self {
        Self {
            alias: std::sync::Mutex::new(alias),
            port,
            fingerprint,
            multicast_addr: MULTICAST_GROUP.parse().unwrap_or(Ipv4Addr::new(224, 0, 0, 167)),
            multicast_port: port, // Use the same port for multicast
        }
    }

    /// Update the alias used in announcements
    pub fn update_alias(&self, new_alias: String) {
        if let Ok(mut alias) = self.alias.lock() {
            *alias = new_alias;
        }
    }

    /// Send an announcement to the multicast group
    /// This triggers other devices to respond with their info
    pub fn send_announcement(&self) -> Result<()> {
        let alias = self.alias.lock().map(|a| a.clone()).unwrap_or_default();
        let announcement = MulticastAnnouncement {
            alias,
            version: crate::PROTOCOL_VERSION.to_string(),
            fingerprint: self.fingerprint.clone(),
            port: self.port,
            os: std::env::consts::OS.to_string(),
            device_type: DeviceType::detect().as_str().to_string(),
            announcement: true,
        };

        let data = announcement.to_bytes();
        if data.is_empty() {
            warn!("Failed to serialize multicast announcement");
            return Ok(());
        }

        // Try to send on all available interfaces
        let interfaces = get_local_interfaces();
        for iface_ip in interfaces {
            match UdpSocket::bind((iface_ip, 0)) {
                Ok(socket) => {
                    socket.set_multicast_ttl_v4(32)?;
                    match socket.send_to(&data, (self.multicast_addr, self.multicast_port)) {
                        Ok(_) => {
                            info!("Sent multicast announcement from {} to {}:{}", iface_ip, self.multicast_addr, self.multicast_port);
                        }
                        Err(e) => {
                            warn!("Failed to send multicast from {}: {}", iface_ip, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to bind UDP socket on {}: {}", iface_ip, e);
                }
            }
        }

        Ok(())
    }

    /// Start listening for multicast announcements from other devices
    /// Returns a receiver channel that emits DiscoveryEvents
    pub fn listen(&self) -> Result<mpsc::UnboundedReceiver<DiscoveryEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let my_fingerprint = self.fingerprint.clone();
        let my_alias = self.alias.lock().map(|a| a.clone()).unwrap_or_default();
        let multicast_addr = self.multicast_addr;
        let multicast_port = self.multicast_port;
        let my_port = self.port;

        // Bind to the multicast port on all interfaces
        // Note: UDP and TCP can coexist on the same port because they are different protocols
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, multicast_port))
            .map_err(|e| {
                info!("Note: If UDP bind fails on port {}, the server will still work but multicast discovery won't be available.", multicast_port);
                e
            })?;
        socket.set_broadcast(true)?;

        // Join multicast group on all interfaces
        let interfaces = get_local_interfaces();
        for iface_ip in interfaces {
            if let Err(e) = socket.join_multicast_v4(&multicast_addr, &iface_ip) {
                warn!("Failed to join multicast group on {}: {}", iface_ip, e);
            } else {
                info!("Joined multicast group on {}:{}", iface_ip, multicast_addr);
            }
        }

        let buf_size = 65535;
        std::thread::spawn(move || {
            let mut buf = vec![0u8; buf_size];
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((len, src_addr)) => {
                        if len == 0 {
                            continue;
                        }

                        match MulticastAnnouncement::from_bytes(&buf[..len]) {
                            Some(announcement) => {
                                // Skip our own announcements
                                if announcement.fingerprint == my_fingerprint {
                                    continue;
                                }

                                let device_type = match announcement.device_type.as_str() {
                                    "mobile" => DeviceType::Mobile,
                                    "web" => DeviceType::Web,
                                    "server" => DeviceType::Server,
                                    _ => DeviceType::Desktop,
                                };

                                let ip = src_addr.ip().to_string();
                                let device = DeviceInfo {
                                    id: announcement.fingerprint.clone(),
                                    alias: announcement.alias,
                                    ip,
                                    port: announcement.port,
                                    version: announcement.version,
                                    device_model: String::new(),
                                    device_type,
                                    fingerprint: announcement.fingerprint,
                                    os: announcement.os,
                                };

                                info!("UDP Multicast: Device found: {} ({})", device.alias, device.ip);

                                if tx.send(DiscoveryEvent::DeviceFound(device)).is_err() {
                                    break; // Channel closed
                                }

                                // If this was an announcement, respond with our own info
                                if announcement.announcement {
                                    respond_to_announcement(
                                        &socket,
                                        multicast_addr,
                                        multicast_port,
                                        &my_alias,
                                        &my_fingerprint,
                                        my_port,
                                    );
                                }
                            }
                            None => {
                                // Not a valid announcement, ignore
                            }
                        }
                    }
                    Err(e) => {
                        warn!("UDP multicast recv error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Respond to a multicast announcement from another device
fn respond_to_announcement(
    socket: &UdpSocket,
    multicast_addr: Ipv4Addr,
    multicast_port: u16,
    my_alias: &str,
    my_fingerprint: &str,
    my_port: u16,
) {
    // We could respond via TCP to the sender's /api/register endpoint,
    // but for simplicity, we respond via multicast so all devices learn about us
    let response = MulticastAnnouncement {
        alias: my_alias.to_string(),
        version: crate::PROTOCOL_VERSION.to_string(),
        fingerprint: my_fingerprint.to_string(),
        port: my_port,
        os: std::env::consts::OS.to_string(),
        device_type: DeviceType::detect().as_str().to_string(),
        announcement: false, // This is a response, not an announcement
    };

    let data = response.to_bytes();
    if !data.is_empty() {
        if let Err(e) = socket.send_to(&data, (multicast_addr, multicast_port)) {
            warn!("Failed to send multicast response: {}", e);
        }
    }
}

/// Get all local IPv4 interface addresses
fn get_local_interfaces() -> Vec<Ipv4Addr> {
    let mut interfaces = Vec::new();

    // Try using local_ip_address crate first
    if let Ok(ip) = local_ip() {
        if let IpAddr::V4(ipv4) = ip {
            interfaces.push(ipv4);
        }
    }

    // Fallback: try to enumerate all interfaces
    #[cfg(target_os = "windows")]
    {
        // On Windows, use ipconfig output as a fallback
        if let Ok(output) = std::process::Command::new("ipconfig").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("IPv4") {
                    if let Some(ip_str) = line.split(':').last() {
                        let ip_str = ip_str.trim().to_string();
                        if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                            if !interfaces.contains(&ip) {
                                interfaces.push(ip);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix, try ifconfig or ip addr
        if let Ok(output) = std::process::Command::new("ifconfig").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && !line.contains("127.0.0.1") {
                    if let Some(ip_str) = line.split("inet ").nth(1) {
                        let ip_str = ip_str.split_whitespace().next().unwrap_or("").to_string();
                        if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                            if !interfaces.contains(&ip) {
                                interfaces.push(ip);
                            }
                        }
                    }
                }
            }
        }
    }

    // Always include the primary local_ip result
    if interfaces.is_empty() {
        interfaces.push(Ipv4Addr::new(0, 0, 0, 0));
    }

    interfaces
}

// ─── mDNS Discovery (supplementary) ──────────────────────────

/// Discovery service using mDNS (supplementary to UDP multicast)
pub struct MdnsDiscovery {
    service_type: String,
    port: u16,
    alias: String,
    fingerprint: String,
    mdns: ServiceDaemon,
}

impl MdnsDiscovery {
    pub fn new(alias: String, port: u16, fingerprint: String) -> Result<Self> {
        let mdns = ServiceDaemon::new()?;
        Ok(Self {
            service_type: SERVICE_TYPE.to_string(),
            port,
            alias,
            fingerprint,
            mdns,
        })
    }

    /// Register this device on the network via mDNS
    pub fn register(&self) -> Result<()> {
        let local_ip = local_ip()?;
        // mDNS instance name must be a valid DNS label: no spaces, lowercase
        let instance_name = format!("quickshare-{}", &self.fingerprint[..8.min(self.fingerprint.len())]);
        let host_name = format!("{}.local.", instance_name);

        let device_type_str = DeviceType::detect().as_str().to_string();

        let properties: Vec<(String, String)> = vec![
            ("alias".to_string(), self.alias.clone()),
            ("version".to_string(), crate::PROTOCOL_VERSION.to_string()),
            ("fingerprint".to_string(), self.fingerprint.clone()),
            ("os".to_string(), std::env::consts::OS.to_string()),
            ("device_type".to_string(), device_type_str),
        ];

        let service_info = ServiceInfo::new(
            &self.service_type,
            &instance_name,
            &host_name,
            local_ip,
            self.port,
            &properties[..],
        )?;

        self.mdns.register(service_info)?;

        info!("Registered mDNS service on {}:{} (alias={})", local_ip, self.port, self.alias);
        Ok(())
    }

    /// Browse for other QuickShare devices on the network via mDNS
    pub fn browse(&self) -> Option<mpsc::UnboundedReceiver<DiscoveryEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let receiver = match self.mdns.browse(&self.service_type) {
            Ok(r) => r,
            Err(e) => {
                warn!("mDNS browse failed: {}", e);
                return None;
            }
        };
        let my_fingerprint = self.fingerprint.clone();

        std::thread::spawn(move || {
            loop {
                match receiver.recv() {
                    Ok(event) => match event {
                        ServiceEvent::ServiceResolved(info) => {
                            let fp = info.get_property_val_str("fingerprint")
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            if fp == my_fingerprint {
                                continue;
                            }

                            let alias = info.get_property_val_str("alias")
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "Unknown".to_string());
                            let port = info.get_port();
                            let addresses = info.get_addresses();
                            let os = info.get_property_val_str("os")
                                .map(|s| s.to_string())
                                .unwrap_or_default();

                            let device_type = info.get_property_val_str("device_type")
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let device_type = match device_type.as_str() {
                                "mobile" => DeviceType::Mobile,
                                "web" => DeviceType::Web,
                                "server" => DeviceType::Server,
                                _ => DeviceType::Desktop,
                            };

                            if let Some(ip) = addresses.iter().next() {
                                let device = DeviceInfo {
                                    id: fp.clone(), // Use fingerprint as stable ID (consistent with UDP multicast)
                                    alias,
                                    ip: ip.to_string(),
                                    port,
                                    version: info.get_property_val_str("version")
                                        .map(|s| s.to_string())
                                        .unwrap_or_default(),
                                    device_model: String::new(),
                                    device_type,
                                    fingerprint: fp,
                                    os,
                                };

                                let _ = tx.send(DiscoveryEvent::DeviceFound(device));
                            }
                        }
                        ServiceEvent::ServiceRemoved(_service_type, fullname) => {
                            let _ = tx.send(DiscoveryEvent::DeviceLost(fullname));
                        }
                        _ => {}
                    },
                    Err(e) => {
                        info!("mDNS browse recv error: {}, stopping", e);
                        break;
                    }
                }
            }
        });

        Some(rx)
    }
}

// ─── Combined Discovery Service ───────────────────────────────

/// Combined discovery service that uses both UDP multicast and mDNS
pub struct DiscoveryService {
    multicast: MulticastDiscovery,
    mdns: Option<MdnsDiscovery>,
}

impl DiscoveryService {
    pub fn new(alias: String, port: u16, fingerprint: String) -> Result<Self> {
        let multicast = MulticastDiscovery::new(alias.clone(), port, fingerprint.clone());
        let mdns = MdnsDiscovery::new(alias, port, fingerprint).ok();

        Ok(Self { multicast, mdns })
    }

    /// Update the alias used in discovery announcements
    pub fn update_alias(&self, new_alias: String) {
        self.multicast.update_alias(new_alias);
    }

    /// Register this device on the network
    pub fn register(&self) -> Result<()> {
        // Register mDNS service (supplementary)
        if let Some(ref mdns) = self.mdns {
            if let Err(e) = mdns.register() {
                info!("Warning: mDNS registration failed (non-critical): {}", e);
            }
        }
        Ok(())
    }

    /// Start browsing for peers (both UDP multicast and mDNS)
    pub fn browse(&self) -> mpsc::UnboundedReceiver<DiscoveryEvent> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Start UDP multicast listener
        if let Ok(multicast_rx) = self.multicast.listen() {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                // We need to bridge std::sync::mpsc to tokio::sync::mpsc
                // Since multicast_rx is tokio::sync::mpsc, we can use it directly
                let mut multicast_rx = multicast_rx;
                while let Some(event) = multicast_rx.recv().await {
                    if tx_clone.send(event).is_err() {
                        break;
                    }
                }
            });
        }

        // Start mDNS browser (supplementary)
        if let Some(ref mdns) = self.mdns {
            if let Some(mdns_rx) = mdns.browse() {
                let tx_clone = tx.clone();
                // mDNS browse returns tokio::sync::mpsc channel
                tokio::spawn(async move {
                    let mut mdns_rx = mdns_rx;
                    while let Some(event) = mdns_rx.recv().await {
                        if tx_clone.send(event).is_err() {
                            break;
                        }
                    }
                });
            }
        }

        rx
    }

    /// Send an announcement to trigger discovery
    /// This should be called when the user clicks "Scan"
    pub fn send_announcement(&self) -> Result<()> {
        self.multicast.send_announcement()
    }
}

/// Get the local IP address
pub fn get_local_ip() -> Result<IpAddr> {
    Ok(local_ip()?)
}
