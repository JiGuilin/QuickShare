use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn, error};

use quickshare_core::protocol::{
    AcceptRequest, CancelRequest, DeviceInfo, InfoResponse,
    PrepareSendRequest, PrepareSendResponse, RejectRequest,
    SettingsRequest, SettingsResponse, TransferProgress, WsMessage,
};
use quickshare_core::transfer::{FileReceiver, TransferSession, TransferStatus};

use crate::state::AppState;

/// GET /api/info - Return device information
pub async fn get_info(State(state): State<AppState>) -> Json<InfoResponse> {
    let device = state.get_device_info().await;
    Json(InfoResponse { device })
}

/// GET /api/devices - Return discovered peer devices
pub async fn get_devices(State(state): State<AppState>) -> Json<Vec<DeviceInfo>> {
    let peers = state.peers.lock().await;
    let devices: Vec<DeviceInfo> = peers.values().cloned().collect();
    Json(devices)
}

/// POST /api/prepare-send - Prepare to receive files from another device
/// Creates a Pending session and pushes a transfer_request via WebSocket.
/// If auto_accept is on, automatically accepts.
pub async fn prepare_send(
    State(state): State<AppState>,
    Json(req): Json<PrepareSendRequest>,
) -> Result<Json<PrepareSendResponse>, StatusCode> {
    info!(
        "Prepare send request from {} ({} files)",
        req.sender.alias,
        req.files.len()
    );

    let total_size: u64 = req.files.iter().map(|f| f.size).sum();
    info!("Total size: {} bytes", total_size);

    let session_id = uuid::Uuid::new_v4().to_string();
    let auto_accept = *state.auto_accept.lock().await;

    let session = TransferSession {
        id: session_id.clone(),
        sender_id: req.sender.id.clone(),
        receiver_id: state.get_device_info().await.id,
        files: req.files.clone(),
        output_dir: Some(state.receive_dir.lock().await.clone()),
        status: if auto_accept { TransferStatus::Accepted } else { TransferStatus::Pending },
        current_file_index: 0,
        bytes_transferred: 0,
        total_bytes: total_size,
    };

    // Ensure receive directory exists
    {
        let dir = state.receive_dir.lock().await.clone();
        if let Err(e) = FileReceiver::ensure_dir(std::path::Path::new(&dir)).await {
            error!("Failed to create receive directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    state
        .sessions
        .lock()
        .await
        .insert(session_id.clone(), session);

    if auto_accept {
        // Auto-accept: notify WS clients and return accepted
        notify_ws(&state, &WsMessage::TransferResponse {
            session_id: session_id.clone(),
            accepted: true,
        }).await;

        Ok(Json(PrepareSendResponse {
            session_id,
            accepted: true,
            output_dir: state.receive_dir.lock().await.clone().into(),
        }))
    } else {
        // Push transfer_request to frontend via WebSocket
        notify_ws(&state, &WsMessage::TransferRequest {
            session_id: session_id.clone(),
            from: req.sender,
            files: req.files,
        }).await;

        // Return a "pending" response - the sender should wait for WS notification
        Ok(Json(PrepareSendResponse {
            session_id,
            accepted: false,
            output_dir: None,
        }))
    }
}

/// POST /api/accept - Accept an incoming transfer
pub async fn accept_transfer(
    State(state): State<AppState>,
    Json(req): Json<AcceptRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get_mut(&req.session_id) {
        if session.status != TransferStatus::Pending {
            warn!("Session {} not pending: {:?}", req.session_id, session.status);
            return Err(StatusCode::CONFLICT);
        }
        session.status = TransferStatus::Accepted;
        info!("Transfer {} accepted", req.session_id);
    } else {
        warn!("Session not found: {}", req.session_id);
        return Err(StatusCode::NOT_FOUND);
    }
    drop(sessions);

    // Notify WebSocket clients (so the sender knows it was accepted)
    notify_ws(&state, &WsMessage::TransferResponse {
        session_id: req.session_id.clone(),
        accepted: true,
    }).await;

    Ok(StatusCode::OK)
}

/// POST /api/reject - Reject an incoming transfer
pub async fn reject_transfer(
    State(state): State<AppState>,
    Json(req): Json<RejectRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get_mut(&req.session_id) {
        if session.status != TransferStatus::Pending {
            return Err(StatusCode::CONFLICT);
        }
        session.status = TransferStatus::Cancelled;
        info!("Transfer {} rejected: {:?}", req.session_id, req.reason);
    } else {
        return Err(StatusCode::NOT_FOUND);
    }
    drop(sessions);

    // Notify WebSocket clients
    notify_ws(&state, &WsMessage::TransferResponse {
        session_id: req.session_id.clone(),
        accepted: false,
    }).await;

    Ok(StatusCode::OK)
}

/// POST /api/send - Receive file data (multipart upload) with streaming write
/// The session_id is passed as a form field alongside the file.
pub async fn send_file(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<StatusCode, StatusCode> {
    let mut session_id: Option<String> = None;
    let mut total_session_bytes: u64 = 0;
    let mut last_progress_time = std::time::Instant::now();

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let field_name = field.name().unwrap_or("").to_string();

        // Handle session_id field
        if field_name == "session_id" {
            let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            session_id = Some(text);
            continue;
        }

        // Handle file field
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        info!("Receiving file: {} ({})", file_name, content_type);

        let output_dir = state.receive_dir.lock().await.clone();
        let file_path = get_unique_path(std::path::PathBuf::from(&output_dir).join(&file_name));

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            if let Err(e) = FileReceiver::ensure_dir(parent).await {
                error!("Failed to create directory: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        // Stream file chunks to disk (avoid OOM on large files)
        let mut file = tokio::fs::File::create(&file_path).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Get file metadata for verification
        let file_meta = {
            let sessions = state.sessions.lock().await;
            session_id.as_ref()
                .and_then(|sid| sessions.get(sid))
                .and_then(|s| s.files.iter().find(|f| f.name == file_name).cloned())
        };

        let mut field = field;
        let mut file_bytes: u64 = 0;
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    if let Err(e) = file.write_all(&chunk).await {
                        error!("Failed to write chunk: {}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                    file_bytes += chunk.len() as u64;
                    total_session_bytes += chunk.len() as u64;

                    // Push progress every 200ms to avoid flooding
                    if let Some(ref sid) = session_id {
                        let now = std::time::Instant::now();
                        if now.duration_since(last_progress_time).as_millis() > 200 {
                            let total = {
                                let sessions = state.sessions.lock().await;
                                sessions.get(sid).map(|s| s.total_bytes).unwrap_or(0)
                            };
                            notify_ws(&state, &WsMessage::Progress {
                                progress: TransferProgress {
                                    session_id: sid.clone(),
                                    file_id: file_name.clone(),
                                    bytes_sent: total_session_bytes,
                                    total_bytes: total,
                                    speed_bps: 0,
                                },
                            }).await;
                            last_progress_time = now;
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    error!("Failed to read chunk: {}", e);
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }

        if let Err(e) = file.flush().await {
            error!("Failed to flush file: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        info!("✅ File saved: {:?} ({} bytes)", file_path, file_bytes);

        // Verify file integrity if SHA256 was provided
        if let Some(ref meta) = file_meta {
            if let Some(ref expected_hash) = meta.sha256 {
                if !FileReceiver::verify_file(&file_path, expected_hash).await {
                    warn!("❌ SHA256 verification failed for: {:?}", file_path);
                    // Don't fail the transfer, just warn - the file is still saved
                }
            }
        }

        // Update session progress
        if let Some(ref sid) = session_id {
            let mut sessions = state.sessions.lock().await;
            if let Some(session) = sessions.get_mut(sid) {
                session.bytes_transferred = total_session_bytes;
                session.current_file_index += 1;
            }
        }
    }

    // Notify WebSocket clients that transfer is complete
    if let Some(ref sid) = session_id {
        // Update session status
        {
            let mut sessions = state.sessions.lock().await;
            if let Some(session) = sessions.get_mut(sid) {
                session.status = TransferStatus::Completed;
            }
        }

        notify_ws(&state, &WsMessage::TransferComplete {
            session_id: sid.clone(),
        }).await;
    } else {
        // Fallback for legacy clients that don't send session_id
        notify_ws(&state, &WsMessage::TransferComplete {
            session_id: "latest".to_string(),
        }).await;
    }

    Ok(StatusCode::OK)
}

/// POST /api/cancel - Cancel a transfer session
pub async fn cancel_transfer(
    State(state): State<AppState>,
    Json(req): Json<CancelRequest>,
) -> StatusCode {
    info!("Cancel transfer: {} - {}", req.session_id, req.reason);

    if let Some(session) = state.sessions.lock().await.get_mut(&req.session_id) {
        session.status = TransferStatus::Cancelled;
        StatusCode::OK
    } else {
        warn!("Session not found: {}", req.session_id);
        StatusCode::NOT_FOUND
    }
}

/// GET /api/settings - Get current settings
pub async fn get_settings(State(state): State<AppState>) -> Json<SettingsResponse> {
    let alias = state.alias.lock().await.clone();
    let download_dir = state.receive_dir.lock().await.clone();
    let auto_accept = *state.auto_accept.lock().await;

    Json(SettingsResponse {
        alias,
        port: state.port,
        download_dir,
        auto_accept,
    })
}

/// POST /api/settings - Update settings
pub async fn update_settings(
    State(state): State<AppState>,
    Json(req): Json<SettingsRequest>,
) -> Result<Json<SettingsResponse>, StatusCode> {
    if let Some(new_alias) = req.alias {
        state.update_alias(new_alias).await;
    }
    if let Some(new_dir) = req.download_dir {
        if let Err(e) = FileReceiver::ensure_dir(std::path::Path::new(&new_dir)).await {
            error!("Failed to create download directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        *state.receive_dir.lock().await = new_dir;
    }
    if let Some(auto) = req.auto_accept {
        *state.auto_accept.lock().await = auto;
    }

    // Persist settings to disk
    state.persist_settings().await;

    let alias = state.alias.lock().await.clone();
    let download_dir = state.receive_dir.lock().await.clone();
    let auto_accept = *state.auto_accept.lock().await;

    Ok(Json(SettingsResponse {
        alias,
        port: state.port,
        download_dir,
        auto_accept,
    }))
}

/// Notify all connected WebSocket clients
async fn notify_ws(state: &AppState, msg: &WsMessage) {
    let clients = state.ws_clients.lock().await;
    for (_, tx) in clients.iter() {
        if let Err(e) = tx.send(msg.clone()) {
            warn!("Failed to send WS message: {}", e);
        }
    }
}

/// Generate a unique file path by appending a number if the file already exists
fn get_unique_path(mut path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }

    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let parent = path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();

    let mut counter = 1u32;
    loop {
        let new_name = format!("{} ({}){}", stem, counter, ext);
        path = parent.join(&new_name);
        if !path.exists() {
            return path;
        }
        counter += 1;
    }
}
