#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod context_menu;

use quickshare_core::DEFAULT_PORT;
use quickshare_server::run_server;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// 存储从命令行传入的文件路径
static CLI_FILES: Mutex<Option<Vec<String>>> = Mutex::new(None);

#[tauri::command]
fn open_dir(path: String) -> Result<(), String> {
    opener::open(&path).map_err(|e| format!("Failed to open directory: {}", e))
}

/// 从命令行获取待发送的文件
#[tauri::command]
fn get_cli_files() -> Vec<String> {
    CLI_FILES.lock().unwrap().take().unwrap_or_default()
}

/// 处理从上下文菜单接收的文件
#[tauri::command]
async fn handle_context_menu_files(files: Vec<String>) -> Result<Vec<context_menu::FileToSend>, String> {
    context_menu::handle_context_menu_files(files).await
}

/// 注册上下文菜单（仅在用户请求时调用）
#[tauri::command]
async fn register_context_menu(app_handle: AppHandle) -> Result<String, String> {
    let app_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get app path: {}", e))?;
    
    let app_path = app_exe.to_str()
        .ok_or("Failed to convert app path to string")?;
    
    context_menu::platform::register_context_menu(app_path)?;
    
    Ok("Context menu registered successfully".to_string())
}

fn main() {
    // 处理命令行参数
    let args: Vec<String> = std::env::args().collect();
    
    // 检查 --quickshare-send 参数
    if let Some(idx) = args.iter().position(|arg| arg == "--quickshare-send") {
        if idx + 1 < args.len() {
            let files: Vec<String> = args[idx + 1..].to_vec();
            *CLI_FILES.lock().unwrap() = Some(files);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            open_dir,
            get_cli_files,
            handle_context_menu_files,
            register_context_menu,
        ])
        .setup(|app| {
            // 如果有命令行参数文件，发送事件到前端
            let cli_files = CLI_FILES.lock().unwrap().clone();
            if let Some(files) = cli_files {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // 延迟发送事件，确保前端已准备好
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    let _ = app_handle.emit("quickshare://context-menu-files", &files);
                });
            }

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
