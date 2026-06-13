use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use quickshare_core::protocol::{DeviceInfo, WsMessage};
use quickshare_core::transfer::TransferSession;

#[derive(Clone)]
pub struct AppState {
    pub alias: Arc<Mutex<String>>,
    pub port: u16,
    pub device_info: Arc<Mutex<DeviceInfo>>,
    pub sessions: Arc<Mutex<HashMap<String, TransferSession>>>,
    pub peers: Arc<Mutex<HashMap<String, DeviceInfo>>>,
    pub ws_clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<WsMessage>>>>,
    pub receive_dir: Arc<Mutex<String>>,
    pub auto_accept: Arc<Mutex<bool>>,
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
            alias: Arc::new(Mutex::new(alias)),
            port,
            device_info: Arc::new(Mutex::new(device_info)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            peers: Arc::new(Mutex::new(HashMap::new())),
            ws_clients: Arc::new(Mutex::new(HashMap::new())),
            receive_dir: Arc::new(Mutex::new(receive_dir)),
            auto_accept: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn get_device_info(&self) -> DeviceInfo {
        let mut device = self.device_info.lock().await.clone();
        // Always sync alias in case it was updated
        let alias = self.alias.lock().await.clone();
        device.alias = alias;
        device
    }

    pub async fn update_alias(&self, new_alias: String) {
        *self.alias.lock().await = new_alias.clone();
        let mut device = self.device_info.lock().await;
        device.alias = new_alias;
    }
}
