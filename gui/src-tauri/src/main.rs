#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod context_menu;
mod single_instance;

use quickshare_core::DEFAULT_PORT;
use quickshare_server::run_server;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// 存储从命令行传入的文件路径
static CLI_FILES: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// 处理上下文菜单文件操作
fn handle_context_menu_files_action(files: Vec<String>, action: &str) {
    let mut files_ref = CLI_FILES.lock().unwrap();
    *files_ref = Some(files);
    println!("Context menu action '{}' triggered with files", action);
}

#[tauri::command]
fn open_dir(path: String) -> Result<(), String> {
    opener::open(&path).map_err(|e| format!("Failed to open directory: {}", e))
}

/// 从命令行获取待发送的文件
#[tauri::command]
fn get_cli_files() -> Vec<String> {
    CLI_FILES.lock().unwrap().take().unwrap_or_default()
}

/// 注册上下文菜单（仅在用户请求时调用）
#[tauri::command]
async fn register_context_menu(_app_handle: AppHandle) -> Result<String, String> {
    let app_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get app path: {}", e))?;
    
    let app_path = app_exe.to_str()
        .ok_or("Failed to convert app path to string")?;
    
    context_menu::platform::register_context_menu(app_path)?;
    
    Ok("Context menu registered successfully".to_string())
}

/// 卸载上下文菜单
#[tauri::command]
async fn unregister_context_menu(_app_handle: AppHandle) -> Result<String, String> {
    context_menu::platform::unregister_context_menu()?;
    
    Ok("Context menu unregistered successfully".to_string())
}

fn main() {
    // 处理命令行参数
    let args: Vec<String> = std::env::args().collect();
    
    let mut context_action: Option<(Vec<String>, String)> = None;

    // 检查 --quickshare-send 参数（发送到 QuickShare）
    if let Some(idx) = args.iter().position(|arg| arg == "--quickshare-send") {
        if idx + 1 < args.len() {
            let files: Vec<String> = args[idx + 1..].to_vec();
            context_action = Some((files, "send".to_string()));
        }
    }
    // 检查 --quickshare-send-multi 参数（发送到多设备）
    else if let Some(idx) = args.iter().position(|arg| arg == "--quickshare-send-multi") {
        if idx + 1 < args.len() {
            let files: Vec<String> = args[idx + 1..].to_vec();
            context_action = Some((files, "send_multi".to_string()));
        }
    }
    // 检查 --quickshare-send-recent 参数（发送到最近设备）
    else if let Some(idx) = args.iter().position(|arg| arg == "--quickshare-send-recent") {
        if idx + 1 < args.len() {
            let files: Vec<String> = args[idx + 1..].to_vec();
            context_action = Some((files, "send_recent".to_string()));
        }
    }
    // 检查 --quickshare-queue 参数（添加到发送队列）
    else if let Some(idx) = args.iter().position(|arg| arg == "--quickshare-queue") {
        if idx + 1 < args.len() {
            let files: Vec<String> = args[idx + 1..].to_vec();
            context_action = Some((files, "queue".to_string()));
        }
    }

    // 如果有右键菜单触发的文件，尝试通过命名管道发送给已运行的实例
    if let Some((files, action)) = &context_action {
        match single_instance::send_to_existing_instance(files, action) {
            Ok(()) => {
                // 成功发送给已运行实例，退出当前进程
                println!("Sent files to existing QuickShare instance, exiting.");
                return;
            }
            Err(_) => {
                // 没有已运行实例或发送失败，将文件存储到全局状态
                // 由当前实例自行处理
                handle_context_menu_files_action(files.clone(), action);
            }
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
            context_menu::handle_context_menu_files,
            register_context_menu,
            unregister_context_menu,
        ])
        .setup(|app| {
            // 启动命名管道服务器，用于接收来自第二个实例的文件
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                single_instance::listen_for_instances(&app_handle).await;
            });

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
