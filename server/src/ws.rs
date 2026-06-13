use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio::sync::mpsc;
use tracing::{info, warn, debug, error};

use quickshare_core::protocol::{DeviceInfo, WsMessage};

use crate::state::AppState;

/// GET /api/ws - WebSocket handler for real-time communication
pub async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(state, socket))
}

async fn handle_socket(state: AppState, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // Assign a temporary ID to this client
    let client_id = uuid::Uuid::new_v4().to_string();

    // Register the client
    {
        state.ws_clients.lock().await.insert(client_id.clone(), tx);
    }
    info!("WebSocket client connected: {}", client_id);

    // Send hello with current device info and known peers
    let device = state.get_device_info().await;
    let peers: Vec<DeviceInfo> = state.peers.lock().await.values().cloned().collect();
    let hello_msg = WsMessage::Hello {
        device: device.clone(),
        peers,
    };
    let serialized = serde_json::to_string(&hello_msg).unwrap();
    if sender.send(Message::Text(serialized.into())).await.is_err() {
        return;
    }

    // Task: Forward messages from channel to WebSocket
    let client_id_clone = client_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let serialized = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(serialized.into())).await.is_err() {
                break;
            }
        }
    });

    // Task: Handle incoming WebSocket messages
    let state_clone = state.clone();
    let client_id_clone2 = client_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<WsMessage>(&text) {
                    Ok(ws_msg) => {
                        debug!("Received WS message: {:?}", ws_msg);
                        handle_ws_message(&state_clone, &client_id_clone2, ws_msg).await;
                    }
                    Err(e) => {
                        warn!("Failed to parse WS message: {}", e);
                    }
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => { recv_task.abort(); },
        _ = &mut recv_task => { send_task.abort(); },
    }

    // Cleanup
    state.ws_clients.lock().await.remove(&client_id);
    info!("WebSocket client disconnected: {}", client_id);
}

async fn handle_ws_message(state: &AppState, client_id: &str, msg: WsMessage) {
    match msg {
        WsMessage::Join { device } => {
            info!("Device joined: {} ({})", device.alias, device.id);
            state.peers.lock().await.insert(device.id.clone(), device.clone());

            // Notify other clients
            let clients = state.ws_clients.lock().await;
            for (id, tx) in clients.iter() {
                if id != client_id {
                    let _ = tx.send(WsMessage::Join { device: device.clone() });
                }
            }
        }
        WsMessage::Update { device } => {
            state.peers.lock().await.insert(device.id.clone(), device.clone());
        }
        WsMessage::TransferResponse { session_id, accepted } => {
            info!("Transfer response: {} accepted={}", session_id, accepted);
        }
        _ => {
            debug!("Unhandled WS message: {:?}", msg);
        }
    }
}
