use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::PROTOCOL_VERSION;

/// Device information exchanged during discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub alias: String,
    pub ip: String,
    pub port: u16,
    pub version: String,
    pub device_model: String,
    pub device_type: DeviceType,
    pub fingerprint: String,
    pub os: String,
}

impl DeviceInfo {
    pub fn new(alias: String, port: u16) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            alias,
            ip: String::new(),
            port,
            version: PROTOCOL_VERSION.to_string(),
            device_model: get_hostname(),
            device_type: DeviceType::Desktop,
            fingerprint: Uuid::new_v4().to_string()[..8].to_string(),
            os: std::env::consts::OS.to_string(),
        }
    }
}

fn get_hostname() -> String {
    let mut buf = [0u8; 256];
    let result = unsafe {
        libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len())
    };
    if result == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..len]).to_string()
    } else {
        String::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Mobile,
    Desktop,
    Web,
    Server,
}

/// API routes
pub mod routes {
    pub const INFO: &str = "/api/info";
    pub const SEND: &str = "/api/send";
    pub const RECEIVE: &str = "/api/receive";
    pub const PREPARE_SEND: &str = "/api/prepare-send";
    pub const CANCEL: &str = "/api/cancel";
    pub const WS: &str = "/api/ws";
}

/// Request: Device asks for info about another device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoRequest {
    pub version: String,
}

/// Response: Device info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoResponse {
    pub device: DeviceInfo,
}

/// Request: Prepare to send files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareSendRequest {
    pub sender: DeviceInfo,
    pub files: Vec<FileMeta>,
    pub pin: Option<String>,
}

/// Response: Receiver accepts or rejects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareSendResponse {
    pub session_id: String,
    pub accepted: bool,
    pub output_dir: Option<String>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub file_type: String,
    pub sha256: Option<String>,
}

/// Transfer progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub session_id: String,
    pub file_id: String,
    pub bytes_sent: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}

/// Cancel request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    pub session_id: String,
    pub reason: String,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "join")]
    Join { device: DeviceInfo },
    #[serde(rename = "hello")]
    Hello { device: DeviceInfo, peers: Vec<DeviceInfo> },
    #[serde(rename = "leave")]
    Leave { device_id: String },
    #[serde(rename = "update")]
    Update { device: DeviceInfo },
    #[serde(rename = "transfer_request")]
    TransferRequest { from: DeviceInfo, files: Vec<FileMeta> },
    #[serde(rename = "transfer_response")]
    TransferResponse { session_id: String, accepted: bool },
    #[serde(rename = "progress")]
    Progress { progress: TransferProgress },
    #[serde(rename = "error")]
    Error { code: u16, message: String },
}
