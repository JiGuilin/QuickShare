mod state;
mod handler;
mod ws;

use anyhow::Result;
use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use quickshare_core::DEFAULT_PORT;
use quickshare_core::discovery::DiscoveryService;
use quickshare_core::protocol::WsMessage;
use state::AppState;

pub use state::AppState as SharedAppState;

pub async fn run_server(port: u16, alias: String) -> Result<()> {
    let state = AppState::new(alias.clone(), port);
    let port = if port == 0 { DEFAULT_PORT } else { port };

    // Get our fingerprint from the persistent settings
    let my_fingerprint = state.fingerprint.clone();

    // mDNS discovery
    let discovery = match DiscoveryService::new(alias.clone(), port, my_fingerprint) {
        Ok(d) => {
            if let Err(e) = d.register() {
                info!("Warning: mDNS registration failed: {}", e);
                None
            } else {
                Some(d)
            }
        }
        Err(e) => {
            info!("Warning: mDNS discovery unavailable: {}", e);
            None
        }
    };

    // Start browsing for peers in background
    if let Some(disc) = discovery {
        let mut rx = disc.browse();
        let state_clone = state.clone();
        tokio::spawn(async move {
            loop {
                match rx.try_recv() {
                    Ok(event) => match event {
                        quickshare_core::discovery::DiscoveryEvent::DeviceFound(device) => {
                            info!("mDNS: Device found: {} ({})", device.alias, device.ip);
                            let id = device.id.clone();
                            state_clone.peers.lock().await.insert(id.clone(), device.clone());

                            // Notify WebSocket clients
                            let clients = state_clone.ws_clients.lock().await;
                            for (_, tx) in clients.iter() {
                                let _ = tx.send(WsMessage::Join { device: device.clone() });
                            }
                        }
                        quickshare_core::discovery::DiscoveryEvent::DeviceLost(id) => {
                            info!("mDNS: Device lost: {}", id);
                            state_clone.peers.lock().await.remove(&id);

                            let clients = state_clone.ws_clients.lock().await;
                            for (_, tx) in clients.iter() {
                                let _ = tx.send(WsMessage::Leave { device_id: id.clone() });
                            }
                        }
                    },
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                }
            }
        });
    }

    let app = Router::new()
        .route("/api/info", get(handler::get_info))
        .route("/api/devices", get(handler::get_devices))
        .route("/api/prepare-send", post(handler::prepare_send))
        .route("/api/accept", post(handler::accept_transfer))
        .route("/api/reject", post(handler::reject_transfer))
        .route("/api/send", post(handler::send_file))
        .route("/api/cancel", post(handler::cancel_transfer))
        .route("/api/settings", get(handler::get_settings).post(handler::update_settings))
        .route("/api/random-alias", get(handler::get_random_alias))
        .route("/api/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("QuickShare server listening on {}", addr);
    info!("Device alias: {}", alias);

    axum::serve(listener, app).await?;

    Ok(())
}
