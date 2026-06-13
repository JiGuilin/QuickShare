use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};
use uuid::Uuid;
use tracing::{info, debug};

use crate::protocol::{FileMeta, PrepareSendRequest, PrepareSendResponse};

/// A file transfer session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSession {
    pub id: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub files: Vec<FileMeta>,
    pub output_dir: Option<String>,
    pub status: TransferStatus,
    pub current_file_index: usize,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
}

impl TransferSession {
    pub fn new(sender_id: String, receiver_id: String, files: Vec<FileMeta>) -> Self {
        let total_bytes = files.iter().map(|f| f.size).sum();
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id,
            receiver_id,
            files,
            output_dir: None,
            status: TransferStatus::Pending,
            current_file_index: 0,
            bytes_transferred: 0,
            total_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Pending,
    Accepted,
    InProgress,
    Completed,
    Cancelled,
    Failed(String),
}

/// File sender
pub struct FileSender;

impl FileSender {
    /// Prepare files for sending - collect metadata
    pub fn prepare_files(paths: &[PathBuf]) -> Result<Vec<FileMeta>> {
        let mut files = Vec::new();
        for path in paths {
            let meta = std::fs::metadata(path)
                .with_context(|| format!("Cannot access file: {:?}", path))?;
            let name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let file_type = path.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            files.push(FileMeta {
                id: Uuid::new_v4().to_string(),
                name,
                size: meta.len(),
                file_type,
                sha256: None,
            });
        }
        Ok(files)
    }

    /// Read a file chunk
    pub async fn read_chunk(path: &Path, offset: u64, chunk_size: usize) -> Result<Vec<u8>> {
        let mut file = File::open(path).await?;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        let mut buffer = vec![0u8; chunk_size];
        let n = file.read(&mut buffer).await?;
        buffer.truncate(n);
        Ok(buffer)
    }
}

/// File receiver
pub struct FileReceiver;

impl FileReceiver {
    /// Write a chunk of data to a file
    pub async fn write_chunk(path: &Path, data: &[u8], append: bool) -> Result<()> {
        let mut file = if append {
            File::options().write(true).append(true).open(path).await?
        } else {
            File::create(path).await?
        };
        file.write_all(data).await?;
        file.flush().await?;
        Ok(())
    }

    /// Ensure output directory exists
    pub async fn ensure_dir(path: &Path) -> Result<()> {
        if !path.exists() {
            tokio::fs::create_dir_all(path).await?;
        }
        Ok(())
    }
}
