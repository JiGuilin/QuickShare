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
use state::AppState;

pub use state::AppState as SharedAppState;

pub async fn run_server(port: u16, alias: String) -> Result<()> {
    let state = AppState::new(alias.clone(), port);
    let port = if port == 0 { DEFAULT_PORT } else { port };

    let app = Router::new()
        // REST API routes
        .route("/api/info", get(handler::get_info))
        .route("/api/prepare-send", post(handler::prepare_send))
        .route("/api/send", post(handler::send_file))
        .route("/api/cancel", post(handler::cancel_transfer))
        // WebSocket for real-time discovery and notifications
        .route("/api/ws", get(ws::ws_handler))
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("🚀 QuickShare server listening on {}", addr);
    info!("Device alias: {}", alias);

    axum::serve(listener, app).await?;

    Ok(())
}
