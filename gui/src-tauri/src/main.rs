#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use quickshare_core::DEFAULT_PORT;
use quickshare_server::run_server;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|_app| {
            let port = DEFAULT_PORT;

            // Start the server in the background
            // Pass empty alias so the server uses the persisted one (or generates a new random one on first run)
            let alias = String::new();
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
