use anyhow::Result;
use std::path::Path;

/// Compute a simple hash of file data (using basic hashing)
pub fn file_hash(data: &[u8]) -> String {
    // Simple FNV-1a hash for demonstration
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Generate a random fingerprint for this device
pub fn generate_fingerprint() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:016x}", ts ^ (ts >> 32))
}

/// Generate a simple session key
pub fn generate_session_key() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id() as u128;
    format!("{:032x}", ts ^ (pid << 64))
}
