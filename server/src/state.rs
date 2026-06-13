use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use quickshare_core::protocol::{DeviceInfo, WsMessage};
use quickshare_core::transfer::TransferSession;
use tracing::{info, warn, error};

/// Persistent settings stored on disk
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentSettings {
    pub alias: String,
    pub download_dir: String,
    pub auto_accept: bool,
    pub fingerprint: String,
}

impl Default for PersistentSettings {
    fn default() -> Self {
        let fingerprint = quickshare_core::crypto::generate_fingerprint();
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("QuickShare")
            .to_string_lossy()
            .to_string();

        Self {
            alias: quickshare_core::alias::generate_random_alias("en"),
            download_dir,
            auto_accept: false,
            fingerprint,
        }
    }
}

fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("QuickShare")
        .join("settings.json")
}

/// Load settings from disk, or create defaults
fn load_settings() -> PersistentSettings {
    let path = config_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<PersistentSettings>(&content) {
                    Ok(settings) => {
                        info!("Loaded settings from {:?}", path);
                        return settings;
                    }
                    Err(e) => {
                        warn!("Failed to parse settings file: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read settings file: {}", e);
            }
        }
    }

    // Create default settings and save
    let settings = PersistentSettings::default();
    save_settings(&settings);
    settings
}

/// Save settings to disk
fn save_settings(settings: &PersistentSettings) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            error!("Failed to create config directory: {}", e);
            return;
        }
    }
    match serde_json::to_string_pretty(settings) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                error!("Failed to save settings: {}", e);
            } else {
                info!("Settings saved to {:?}", path);
            }
        }
        Err(e) => {
            error!("Failed to serialize settings: {}", e);
        }
    }
}

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
    pub fingerprint: String,
}

impl AppState {
    pub fn new(alias: String, port: u16) -> Self {
        // Load persisted settings (alias, fingerprint, etc.)
        let mut persisted = load_settings();

        // Only override persisted alias if a non-empty, non-default alias was provided.
        // The GUI passes a random alias as default - we prefer the persisted one.
        // The CLI --alias flag provides an explicit override.
        if !alias.is_empty() {
            persisted.alias = alias;
        }

        let device_info = DeviceInfo::with_fingerprint(
            persisted.alias.clone(),
            port,
            persisted.fingerprint.clone(),
        );

        Self {
            alias: Arc::new(Mutex::new(persisted.alias)),
            port,
            device_info: Arc::new(Mutex::new(device_info)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            peers: Arc::new(Mutex::new(HashMap::new())),
            ws_clients: Arc::new(Mutex::new(HashMap::new())),
            receive_dir: Arc::new(Mutex::new(persisted.download_dir)),
            auto_accept: Arc::new(Mutex::new(persisted.auto_accept)),
            fingerprint: persisted.fingerprint,
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
        device.alias = new_alias.clone();
        self.persist_settings().await;
    }

    /// Persist current settings to disk
    pub async fn persist_settings(&self) {
        let settings = PersistentSettings {
            alias: self.alias.lock().await.clone(),
            download_dir: self.receive_dir.lock().await.clone(),
            auto_accept: *self.auto_accept.lock().await,
            fingerprint: self.fingerprint.clone(),
        };
        // Save in a blocking task to avoid holding locks across await
        let _ = tokio::task::spawn_blocking(move || {
            save_settings(&settings);
        }).await;
    }
}
