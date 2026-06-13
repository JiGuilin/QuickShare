pub mod alias;
pub mod discovery;
pub mod protocol;
pub mod transfer;
pub mod crypto;

pub const PROTOCOL_VERSION: &str = "1.0";
pub const DEFAULT_PORT: u16 = 53318;
pub const SERVICE_TYPE: &str = "_quickshare._tcp.local.";
