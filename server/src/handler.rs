use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tokio::fs;
use tracing::{info, warn, error};

use quickshare_core::protocol::{
    CancelRequest, InfoResponse,
    PrepareSendRequest, PrepareSendResponse,
};
use quickshare_core::transfer::{FileReceiver, TransferSession, TransferStatus};

use crate::state::AppState;

/// GET /api/info - Return device information
pub async fn get_info(State(state): State<AppState>) -> Json<InfoResponse> {
    let device = state.get_device_info().await;
    Json(InfoResponse { device })
}

/// POST /api/prepare-send - Prepare to receive files from another device
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

    // Auto-accept for now (in a real app, this would show a confirmation dialog)
    let session_id = uuid::Uuid::new_v4().to_string();

    let session = TransferSession {
        id: session_id.clone(),
        sender_id: req.sender.id.clone(),
        receiver_id: state.get_device_info().await.id,
        files: req.files.clone(),
        output_dir: Some(state.receive_dir.lock().await.clone()),
        status: TransferStatus::Accepted,
        current_file_index: 0,
        bytes_transferred: 0,
        total_bytes: total_size,
    };

    // Ensure receive directory exists
    if let Some(dir) = &session.output_dir {
        if let Err(e) = FileReceiver::ensure_dir(std::path::Path::new(dir)).await {
            error!("Failed to create receive directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    state
        .sessions
        .lock()
        .await
        .insert(session_id.clone(), session);

    // Notify WebSocket clients
    notify_ws(&state, &quickshare_core::protocol::WsMessage::TransferResponse {
        session_id: session_id.clone(),
        accepted: true,
    }).await;

    Ok(Json(PrepareSendResponse {
        session_id,
        accepted: true,
        output_dir: state.receive_dir.lock().await.clone().into(),
    }))
}

/// POST /api/send - Receive file data (multipart upload)
pub async fn send_file(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<StatusCode, StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        info!("Receiving file: {} ({})", file_name, content_type);

        let output_dir = state.receive_dir.lock().await.clone();
        let file_path = std::path::PathBuf::from(&output_dir).join(&file_name);

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            if let Err(e) = FileReceiver::ensure_dir(parent).await {
                error!("Failed to create directory: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        // Write file chunks
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        if let Err(e) = fs::write(&file_path, &data).await {
            error!("Failed to write file: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        info!("✅ File saved: {:?}", file_path);
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

/// Notify all connected WebSocket clients
async fn notify_ws(state: &AppState, msg: &quickshare_core::protocol::WsMessage) {
    let clients = state.ws_clients.lock().await;
    for (_, tx) in clients.iter() {
        if let Err(e) = tx.send(msg.clone()) {
            warn!("Failed to send WS message: {}", e);
        }
    }
}
