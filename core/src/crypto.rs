use anyhow::Result;
use sha2::{Sha256, Digest};
use std::path::Path;
use tokio::io::AsyncReadExt;

/// Compute SHA256 hash of file data (streaming, memory-efficient)
pub async fn sha256_file(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Compute SHA256 hash of arbitrary bytes
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Generate a stable fingerprint for this device.
/// Uses machine-id on Linux, IOPlatformUUID on macOS, or falls back to hostname+username hash.
/// The result is persisted in config so it survives restarts.
pub fn generate_fingerprint() -> String {
    // Try to get a stable identifier based on platform
    let raw = get_machine_id();
    sha256_bytes(raw.as_bytes())
}

/// Get a machine-specific identifier string
fn get_machine_id() -> String {
    // Try /etc/machine-id on Linux
    if cfg!(target_os = "linux") {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            return id.trim().to_string();
        }
    }

    // On macOS, use IOPlatformUUID via ioreg
    if cfg!(target_os = "macos") {
        if let Ok(output) = std::process::Command::new("ioreg")
            .arg("-rd1")
            .arg("-c")
            .arg("IOPlatformExpertDevice")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("IOPlatformUUID") {
                    if let Some(uuid) = line.split('"').nth(3) {
                        return uuid.to_string();
                    }
                }
            }
        }
    }

    // Fallback: hostname + username
    let hostname = whoami::fallible::hostname().unwrap_or_default();
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    format!("{}@{}", username, hostname)
}
