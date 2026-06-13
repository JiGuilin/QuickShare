use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use quickshare_core::protocol::{DeviceInfo, WsMessage};
use quickshare_core::transfer::TransferSession;

#[derive(Clone)]
pub struct AppState {
    pub alias: String,
    pub port: u16,
    pub device_info: Arc<Mutex<DeviceInfo>>,
    pub sessions: Arc<Mutex<HashMap<String, TransferSession>>>,
    pub peers: Arc<Mutex<HashMap<String, DeviceInfo>>>,
    pub ws_clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<WsMessage>>>>,
    pub receive_dir: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new(alias: String, port: u16) -> Self {
        let device_info = DeviceInfo::new(alias.clone(), port);
        let receive_dir = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("QuickShare")
            .to_string_lossy()
            .to_string();

        Self {
            alias,
            port,
            device_info: Arc::new(Mutex::new(device_info)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            peers: Arc::new(Mutex::new(HashMap::new())),
            ws_clients: Arc::new(Mutex::new(HashMap::new())),
            receive_dir: Arc::new(Mutex::new(receive_dir)),
        }
    }

    pub async fn get_device_info(&self) -> DeviceInfo {
        self.device_info.lock().await.clone()
    }
}
