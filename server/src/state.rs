use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use quickshare_core::discovery::DiscoveryService;
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
    #[serde(default)]
    pub locale: Option<String>,
}

impl Default for PersistentSettings {
    fn default() -> Self {
        let fingerprint = quickshare_core::crypto::generate_fingerprint();
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("QuickShare")
            .to_string_lossy()
            .to_string();

        // Detect system locale for initial alias generation
        let locale = detect_system_locale();

        Self {
            alias: quickshare_core::alias::generate_random_alias(&locale),
            download_dir,
            auto_accept: false,
            fingerprint,
            locale: Some(locale),
        }
    }
}

/// Detect the system locale to decide whether to generate a Chinese or English alias.
/// Returns "zh" for Chinese systems, "en" for everything else.
fn detect_system_locale() -> String {
    // Check LANG, LC_ALL, LC_MESSAGES environment variables (Unix)
    for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if val.to_lowercase().starts_with("zh") {
                return "zh".to_string();
            }
        }
    }

    // On macOS, check AppleLocale defaults
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .arg("read")
            .arg("-g")
            .arg("AppleLocale")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.to_lowercase().starts_with("zh") {
                return "zh".to_string();
            }
        }
    }

    // On Windows, check system UI language via PowerShell
    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to get the system UI language (e.g. "zh-CN", "en-US")
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        if let Ok(output) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "[System.Globalization.CultureInfo]::CurrentUICulture.Name"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
            if stdout.starts_with("zh") {
                return "zh".to_string();
            }
        }

        // Fallback: check environment variables that Windows may set
        for var in &["LANG", "LC_ALL", "LC_MESSAGES"] {
            if let Ok(val) = std::env::var(var) {
                if val.to_lowercase().starts_with("zh") {
                    return "zh".to_string();
                }
            }
        }
    }

    "en".to_string()
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
                    Ok(mut settings) => {
                        info!("Loaded settings from {:?}", path);
                        // If locale is missing (old config), regenerate alias based on system locale
                        if settings.locale.is_none() {
                            let locale = detect_system_locale();
                            info!("Old config without locale detected, regenerating alias for locale: {}", locale);
                            settings.alias = quickshare_core::alias::generate_random_alias(&locale);
                            settings.locale = Some(locale);
                            save_settings(&settings);
                        }
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
    pub discovery: Arc<std::sync::Mutex<Option<DiscoveryService>>>,
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
            discovery: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn set_discovery(&self, discovery: DiscoveryService) {
        *self.discovery.lock().unwrap() = Some(discovery);
    }

    /// Trigger a network scan by sending multicast announcements
    pub fn trigger_scan(&self) -> Result<(), String> {
        let guard = self.discovery.lock().unwrap();
        if let Some(ref disc) = *guard {
            disc.send_announcement().map_err(|e| e.to_string())
        } else {
            Err("Discovery service not available".to_string())
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
        drop(device);
        self.persist_settings().await;

        // Re-announce via multicast so other devices see the updated alias
        self.announce_update(&new_alias);

        // Notify local WebSocket clients about the device info change
        let updated_device = self.get_device_info().await;
        let clients = self.ws_clients.lock().await;
        for (_, tx) in clients.iter() {
            let _ = tx.send(WsMessage::Update { device: updated_device.clone() });
        }
    }

    /// Send a multicast announcement to notify other devices of our updated info
    fn announce_update(&self, new_alias: &str) {
        let guard = self.discovery.lock().unwrap();
        if let Some(ref disc) = *guard {
            // Update the alias in the discovery service so announcements use the new name
            disc.update_alias(new_alias.to_string());
            if let Err(e) = disc.send_announcement() {
                warn!("Failed to re-announce after alias update: {}", e);
            }
        }
    }

    /// Persist current settings to disk
    pub async fn persist_settings(&self) {
        // Determine locale: use the stored one, or detect from system
        let locale = {
            let config_path = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("QuickShare")
                .join("settings.json");
            // Try to read locale from existing config
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                serde_json::from_str::<serde_json::Value>(&content)
                    .ok()
                    .and_then(|v| v.get("locale").and_then(|l| l.as_str()).map(|s| s.to_string()))
            } else {
                None
            }
        }.unwrap_or_else(|| detect_system_locale());

        let settings = PersistentSettings {
            alias: self.alias.lock().await.clone(),
            download_dir: self.receive_dir.lock().await.clone(),
            auto_accept: *self.auto_accept.lock().await,
            fingerprint: self.fingerprint.clone(),
            locale: Some(locale),
        };
        // Save in a blocking task to avoid holding locks across await
        let _ = tokio::task::spawn_blocking(move || {
            save_settings(&settings);
        }).await;
    }
}
