/// 单实例管理模块
/// 
/// 使用命名管道（Named Pipe）实现单实例模式：
/// - 第一个实例启动时创建命名管道服务器，监听来自其他实例的消息
/// - 第二个实例启动时，如果检测到已有实例，通过管道发送文件信息后退出
/// - 第一个实例收到消息后，恢复窗口并聚焦，通过 Tauri 事件将文件传递给前端

use std::io::Write;
use tauri::{AppHandle, Emitter, Manager};

/// 管道名称
#[cfg(target_os = "windows")]
const PIPE_NAME: &str = r"\\.\pipe\quickshare_single_instance";

#[cfg(unix)]
const PIPE_NAME: &str = "/tmp/quickshare_single_instance.sock";

/// 消息格式：JSON 序列化的 SingleInstanceMessage
#[derive(serde::Serialize, serde::Deserialize)]
struct SingleInstanceMessage {
    /// 文件路径列表
    files: Vec<String>,
    /// 操作类型: send, send_multi, send_recent, queue
    action: String,
}

/// 尝试连接到已运行的实例并发送文件信息
/// 成功发送返回 Ok(())，表示已有实例在运行
/// 失败返回 Err，表示没有已运行的实例
pub fn send_to_existing_instance(files: &[String], action: &str) -> Result<(), String> {
    let message = SingleInstanceMessage {
        files: files.to_vec(),
        action: action.to_string(),
    };
    let json = serde_json::to_string(&message)
        .map_err(|e| format!("Failed to serialize message: {}", e))?;

    connect_and_send(&json)
}

#[cfg(target_os = "windows")]
fn connect_and_send(json: &str) -> Result<(), String> {
    // 尝试连接到命名管道
    let mut pipe = std::fs::OpenOptions::new()
        .write(true)
        .open(PIPE_NAME)
        .map_err(|e| format!("No existing instance: {}", e))?;

    // 发送消息（以换行符结尾）
    pipe.write_all(format!("{}\n", json).as_bytes())
        .map_err(|e| format!("Failed to send message: {}", e))?;

    Ok(())
}

#[cfg(unix)]
fn connect_and_send(json: &str) -> Result<(), String> {
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(PIPE_NAME)
        .map_err(|e| format!("No existing instance: {}", e))?;

    stream.write_all(format!("{}\n", json).as_bytes())
        .map_err(|e| format!("Failed to send message: {}", e))?;

    Ok(())
}

/// 将窗口恢复并带到前台
fn bring_window_to_front(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        // 取消最小化
        let _ = window.unminimize();
        // 显示窗口（如果被隐藏）
        let _ = window.show();
        // 聚焦窗口
        let _ = window.set_focus();
    }
}

/// 启动命名管道服务器，持续监听来自其他实例的连接
/// 当收到消息时，恢复窗口并聚焦，通过 Tauri 事件传递给前端
pub async fn listen_for_instances(app_handle: &AppHandle) {
    loop {
        match create_and_listen(app_handle).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Pipe server error: {}, restarting...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

#[cfg(target_os = "windows")]
async fn create_and_listen(app_handle: &AppHandle) -> Result<(), String> {
    use tokio::net::windows::named_pipe::ServerOptions;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    loop {
        // 创建命名管道服务器
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(PIPE_NAME)
            .map_err(|e| format!("Failed to create pipe: {}", e))?;

        // 等待客户端连接
        server.connect().await
            .map_err(|e| format!("Pipe connect failed: {}", e))?;

        // 读取客户端发送的消息
        let (reader, mut writer) = tokio::io::split(server);
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        match buf_reader.read_line(&mut line).await {
            Ok(0) | Err(_) => {
                // 连接关闭或读取错误，继续等待下一个连接
                continue;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Ok(msg) = serde_json::from_str::<SingleInstanceMessage>(trimmed) {
                        println!("Received from secondary instance: {} files, action: {}", 
                            msg.files.len(), msg.action);
                        
                        // 在主线程上恢复窗口并聚焦
                        let app_handle_clone = app_handle.clone();
                        let _ = app_handle.run_on_main_thread(move || {
                            bring_window_to_front(&app_handle_clone);
                        });

                        // 通过 Tauri 事件传递给前端
                        let _ = app_handle.emit("quickshare://context-menu-files", &msg.files);
                    }
                }
            }
        }

        // 发送响应
        let _ = writer.write_all(b"ok\n").await;
        let _ = writer.flush().await;
    }
}

#[cfg(unix)]
async fn create_and_listen(app_handle: &AppHandle) -> Result<(), String> {
    use tokio::net::UnixListener;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    // 清理旧的 socket 文件
    let _ = std::fs::remove_file(PIPE_NAME);

    let listener = UnixListener::bind(PIPE_NAME)
        .map_err(|e| format!("Failed to create unix socket: {}", e))?;

    loop {
        let (stream, _) = listener.accept().await
            .map_err(|e| format!("Failed to accept connection: {}", e))?;

        let (reader, mut writer) = tokio::io::split(stream);
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        match buf_reader.read_line(&mut line).await {
            Ok(0) | Err(_) => continue,
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Ok(msg) = serde_json::from_str::<SingleInstanceMessage>(trimmed) {
                        println!("Received from secondary instance: {} files, action: {}", 
                            msg.files.len(), msg.action);
                        
                        // 在主线程上恢复窗口并聚焦
                        let app_handle_clone = app_handle.clone();
                        let _ = app_handle.run_on_main_thread(move || {
                            bring_window_to_front(&app_handle_clone);
                        });

                        // 通过 Tauri 事件传递给前端
                        let _ = app_handle.emit("quickshare://context-menu-files", &msg.files);
                    }
                }
            }
        }

        let _ = writer.write_all(b"ok\n").await;
        let _ = writer.flush().await;
    }
}
