use std::path::PathBuf;
use tauri::command;

/// 代表一个发送操作的文件
#[derive(Debug, Clone)]
pub struct FileToSend {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// 处理从上下文菜单接收的文件
/// 这个命令会被从 Tauri 前端调用
#[command]
pub async fn handle_context_menu_files(
    files: Vec<String>,
) -> Result<Vec<FileToSend>, String> {
    let mut result = Vec::new();

    for file_path in files {
        let path = PathBuf::from(&file_path);
        
        // 检查文件是否存在
        if !path.exists() {
            return Err(format!("File not found: {}", file_path));
        }

        // 获取文件信息
        let metadata = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to read file metadata: {}", e))?;

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        result.push(FileToSend {
            path,
            name,
            size: metadata.len(),
        });
    }

    Ok(result)
}

/// 平台特定的上下文菜单注册
pub mod platform {
    use super::*;

    /// Windows: 使用注册表添加右键菜单项
    #[cfg(target_os = "windows")]
    pub fn register_context_menu(app_path: &str) -> Result<(), String> {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        
        // 为文件添加右键菜单
        let (key, _) = hkcu.create_subkey(
            r#"Software\Classes\*\shell\QuickShare"#
        ).map_err(|e| e.to_string())?;
        
        key.set_value("", &"Send to QuickShare")
            .map_err(|e| e.to_string())?;
        key.set_value("Icon", &format!("\"{}\"", app_path))
            .map_err(|e| e.to_string())?;

        let (cmd_key, _) = hkcu.create_subkey(
            r#"Software\Classes\*\shell\QuickShare\command"#
        ).map_err(|e| e.to_string())?;
        
        cmd_key.set_value("", &format!(
            "\"{}\" --quickshare-send \"%1\"",
            app_path
        )).map_err(|e| e.to_string())?;

        // 为文件夹添加右键菜单
        let (folder_key, _) = hkcu.create_subkey(
            r#"Software\Classes\Folder\shell\QuickShare"#
        ).map_err(|e| e.to_string())?;
        
        folder_key.set_value("", &"Send to QuickShare")
            .map_err(|e| e.to_string())?;
        folder_key.set_value("Icon", &format!("\"{}\"", app_path))
            .map_err(|e| e.to_string())?;

        let (folder_cmd_key, _) = hkcu.create_subkey(
            r#"Software\Classes\Folder\shell\QuickShare\command"#
        ).map_err(|e| e.to_string())?;
        
        folder_cmd_key.set_value("", &format!(
            "\"{}\" --quickshare-send \"%1\"",
            app_path
        )).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// macOS: 使用 Services 和 plist 添加右键菜单
    #[cfg(target_os = "macos")]
    pub fn register_context_menu(app_path: &str) -> Result<(), String> {
        // macOS 的上下文菜单通常通过应用的 Info.plist 配置
        // 这需要在应用构建时完成，而不是运行时
        // 这里只是占位符
        eprintln!("macOS context menu registration should be done at build time via Info.plist");
        Ok(())
    }

    /// Linux: 使用 Desktop Entry 和 Nautilus Actions
    #[cfg(target_os = "linux")]
    pub fn register_context_menu(app_path: &str) -> Result<(), String> {
        use std::fs;
        use std::path::Path;

        // 创建 Nautilus Desktop Actions
        let actions_dir = dirs::home_dir()
            .ok_or("Could not find home directory")?
            .join(".local/share/nautilus/scripts");

        fs::create_dir_all(&actions_dir)
            .map_err(|e| e.to_string())?;

        let script_path = actions_dir.join("Send to QuickShare");
        let script_content = format!(
            "#!/bin/bash\n\"{}\" --quickshare-send \"$@\"\n",
            app_path
        );

        fs::write(&script_path, script_content)
            .map_err(|e| e.to_string())?;

        // 设置执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&script_path, perms)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// 未支持的平台
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn register_context_menu(_app_path: &str) -> Result<(), String> {
        Err("Context menu registration not supported on this platform".to_string())
    }
}
