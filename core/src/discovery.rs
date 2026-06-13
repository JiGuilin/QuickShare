use anyhow::Result;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::IpAddr;
use tokio::sync::mpsc;
use tracing::info;

use crate::protocol::DeviceInfo;
use crate::SERVICE_TYPE;

/// Discovery service using mDNS
pub struct DiscoveryService {
    service_type: String,
    port: u16,
    alias: String,
    mdns: ServiceDaemon,
}

impl DiscoveryService {
    pub fn new(alias: String, port: u16) -> Result<Self> {
        let mdns = ServiceDaemon::new()?;
        Ok(Self {
            service_type: SERVICE_TYPE.to_string(),
            port,
            alias,
            mdns,
        })
    }

    /// Register this device on the network via mDNS
    pub fn register(&self) -> Result<()> {
        let local_ip = local_ip()?;
        let host_name = format!("{}.local.", self.alias.to_lowercase().replace(' ', "-"));

        let properties: Vec<(String, String)> = vec![
            ("alias".to_string(), self.alias.clone()),
            ("version".to_string(), crate::PROTOCOL_VERSION.to_string()),
        ];

        let service_info = ServiceInfo::new(
            &self.service_type,
            &self.alias,
            &host_name,
            local_ip,
            self.port,
            &properties[..],
        )?;

        self.mdns.register(service_info)?;

        info!("Registered mDNS service on {}:{} (alias={})", local_ip, self.port, self.alias);
        Ok(())
    }

    /// Browse for other QuickShare devices on the network
    pub fn browse(&self) -> mpsc::UnboundedReceiver<DiscoveryEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        let receiver = self.mdns.browse(&self.service_type).unwrap();

        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let alias = info.get_property_val_str("alias")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "Unknown".to_string());
                        let port = info.get_port();
                        let addresses = info.get_addresses();

                        if let Some(ip) = addresses.iter().next() {
                            let device = DeviceInfo {
                                id: info.get_fullname().to_string(),
                                alias,
                                ip: ip.to_string(),
                                port,
                                version: info.get_property_val_str("version")
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                device_model: String::new(),
                                device_type: crate::protocol::DeviceType::Desktop,
                                fingerprint: String::new(),
                                os: String::new(),
                            };

                            let _ = tx.send(DiscoveryEvent::DeviceFound(device));
                        }
                    }
                    ServiceEvent::ServiceRemoved(_service_type, fullname) => {
                        let _ = tx.send(DiscoveryEvent::DeviceLost(fullname));
                    }
                    _ => {}
                }
            }
        });

        rx
    }
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    DeviceFound(DeviceInfo),
    DeviceLost(String),
}

/// Get the local IP address
pub fn get_local_ip() -> Result<IpAddr> {
    Ok(local_ip()?)
}
