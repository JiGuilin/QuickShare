#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use quickshare_core::DEFAULT_PORT;
use quickshare_server::run_server;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            let alias = quickshare_core::alias::generate_random_alias("en");
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
