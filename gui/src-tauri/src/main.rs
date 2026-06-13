#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use quickshare_core::DEFAULT_PORT;
use quickshare_server::run_server;

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            let alias = whoami::fallible::hostname().unwrap_or_else(|_| "QuickShare".to_string());
            let port = DEFAULT_PORT;

            // Start the server in the background
            tauri::async_runtime::spawn(async move {
                if let Err(e) = run_server(port, alias).await {
                    eprintln!("Server error: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
