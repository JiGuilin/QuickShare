use std::path::PathBuf;
use tauri::command;

/// 代表一个发送操作的文件
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileToSend {
    pub path: String,
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
            path: file_path,
            name,
            size: metadata.len(),
        });
    }

    Ok(result)
}

/// 平台特定的上下文菜单注册与卸载
pub mod platform {

    /// Windows: 使用注册表添加右键菜单项（级联子菜单方式）
    ///
    /// 注册表结构：
    /// HKCU\Software\Classes\*\shell\QuickShare          <- 文件右键菜单入口
    ///   (Default) = "QuickShare"
    ///   MUIVerb = "QuickShare"
    ///   Icon = "<app_path>"
    ///   SubCommands = ""                                 <- 必须为空（表示使用 ExtendedSubCommandsKey）
    ///   ExtendedSubCommandsKey = "QuickShare.Menu.File"  <- 指向子命令存储位置
    ///
    /// HKCU\Software\Classes\Folder\shell\QuickShare      <- 文件夹右键菜单入口
    ///   (同上)
    ///   ExtendedSubCommandsKey = "QuickShare.Menu.Folder"
    ///
    /// HKCU\Software\Classes\QuickShare.Menu.File         <- 文件子命令存储位置
    ///   shell\send        -> "发送到 QuickShare"
    ///   shell\sendmulti    -> "发送到多设备..."
    ///   shell\sendrecent   -> "发送到最近设备"
    ///   shell\queue        -> "添加到发送队列"
    ///
    /// HKCU\Software\Classes\QuickShare.Menu.Folder       <- 文件夹子命令存储位置
    ///   (同上)
    #[cfg(target_os = "windows")]
    pub fn register_context_menu(app_path: &str) -> Result<(), String> {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 注册子命令存储区（文件和文件夹各自独立）
        register_submenu_store(&hkcu, app_path, "QuickShare.Menu.File")?;
        register_submenu_store(&hkcu, app_path, "QuickShare.Menu.Folder")?;

        // 为文件创建右键菜单入口
        register_cascading_entry(&hkcu, app_path, "*", "QuickShare.Menu.File")?;
        
        // 为文件夹创建右键菜单入口
        register_cascading_entry(&hkcu, app_path, "Folder", "QuickShare.Menu.Folder")?;

        Ok(())
    }

    /// Windows: 卸载右键菜单
    #[cfg(target_os = "windows")]
    pub fn unregister_context_menu() -> Result<(), String> {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 删除文件右键菜单入口
        let file_menu_path = r"Software\Classes\*\shell\QuickShare";
        if let Err(e) = hkcu.delete_subkey_all(file_menu_path) {
            let err_str = e.to_string();
            if !err_str.contains("cannot find") && !err_str.contains("系统找不到") {
                eprintln!("删除文件右键菜单入口失败: {}", e);
            }
        }

        // 删除文件夹右键菜单入口
        let folder_menu_path = r"Software\Classes\Folder\shell\QuickShare";
        if let Err(e) = hkcu.delete_subkey_all(folder_menu_path) {
            let err_str = e.to_string();
            if !err_str.contains("cannot find") && !err_str.contains("系统找不到") {
                eprintln!("删除文件夹右键菜单入口失败: {}", e);
            }
        }

        // 删除子命令存储区
        let submenu_stores = ["QuickShare.Menu.File", "QuickShare.Menu.Folder"];
        for store in &submenu_stores {
            let store_path = format!(r"Software\Classes\{}", store);
            if let Err(e) = hkcu.delete_subkey_all(&store_path) {
                let err_str = e.to_string();
                if !err_str.contains("cannot find") && !err_str.contains("系统找不到") {
                    eprintln!("删除子命令存储区失败 ({}): {}", store, e);
                }
            }
        }

        Ok(())
    }

    /// 创建级联菜单入口
    /// 在 HKCU\Software\Classes\<type_key>\shell\QuickShare 下创建菜单入口
    /// 指向 ExtendedSubCommandsKey 指定的子命令存储区
    #[cfg(target_os = "windows")]
    fn register_cascading_entry(
        hkcu: &winreg::RegKey,
        app_path: &str,
        type_key: &str,
        subcommands_key: &str,
    ) -> Result<(), String> {
        let shell_key_path = match type_key {
            "*" => r#"Software\Classes\*\shell\QuickShare"#.to_string(),
            "Folder" => r#"Software\Classes\Folder\shell\QuickShare"#.to_string(),
            _ => return Err(format!("Unsupported type_key: {}", type_key)),
        };

        let (shell_key, _) = hkcu.create_subkey(&shell_key_path)
            .map_err(|e| format!("创建菜单入口失败 ({}): {}", type_key, e))?;

        // (Default) 值设置菜单显示名称
        shell_key.set_value("", &"QuickShare")
            .map_err(|e| e.to_string())?;
        // MUIVerb 显示的菜单文本（优先于 Default）
        shell_key.set_value("MUIVerb", &"QuickShare")
            .map_err(|e| e.to_string())?;
        // 图标
        shell_key.set_value("Icon", &app_path)
            .map_err(|e| e.to_string())?;
        // SubCommands 必须为空字符串，表示使用 ExtendedSubCommandsKey
        shell_key.set_value("SubCommands", &"")
            .map_err(|e| e.to_string())?;
        // 指向子命令存储的注册表位置
        shell_key.set_value("ExtendedSubCommandsKey", &subcommands_key)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 注册子命令存储区
    /// 在 HKCU\Software\Classes\<store_name>\shell\<command_id>\command 下创建各子命令
    #[cfg(target_os = "windows")]
    fn register_submenu_store(
        hkcu: &winreg::RegKey,
        app_path: &str,
        store_name: &str,
    ) -> Result<(), String> {
        let store_path = format!(r"Software\Classes\{}", store_name);

        // 子菜单项定义: (command_id, 显示名称, 命令行参数)
        let subcommands = [
            ("send", "发送到 QuickShare", "--quickshare-send"),
            ("sendmulti", "发送到多设备...", "--quickshare-send-multi"),
            ("sendrecent", "发送到最近设备", "--quickshare-send-recent"),
            ("queue", "添加到发送队列", "--quickshare-queue"),
        ];

        for (command_id, display_name, arg) in &subcommands {
            let shell_path = format!(r"{}\shell\{}", store_path, command_id);
            
            let (shell_key, _) = hkcu.create_subkey(&shell_path)
                .map_err(|e| format!("创建子命令项失败 ({}.{}): {}", store_name, command_id, e))?;

            // 设置子菜单显示文本
            shell_key.set_value("", &*display_name)
                .map_err(|e| e.to_string())?;
            // 设置图标
            shell_key.set_value("Icon", &app_path)
                .map_err(|e| e.to_string())?;

            // 创建 command 子键
            let command_path = format!(r"{}\command", shell_path);
            let (command_key, _) = hkcu.create_subkey(&command_path)
                .map_err(|e| format!("创建命令项失败 ({}.{}): {}", store_name, command_id, e))?;

            // 设置执行的命令
            command_key.set_value("", &format!(
                "\"{}\" {} \"%1\"",
                app_path, arg
            )).map_err(|e| e.to_string())?;
        }

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

    /// macOS: 卸载右键菜单
    #[cfg(target_os = "macos")]
    pub fn unregister_context_menu() -> Result<(), String> {
        eprintln!("macOS context menu unregistration should be done via removing the app");
        Ok(())
    }

    /// Linux: 使用 Desktop Entry 和 Nautilus Actions
    #[cfg(target_os = "linux")]
    pub fn register_context_menu(app_path: &str) -> Result<(), String> {
        use std::fs;

        // 创建 Nautilus Desktop Actions
        let quickshare_dir = dirs::home_dir()
            .ok_or("Could not find home directory")?
            .join(".local/share/nautilus/scripts/QuickShare");

        fs::create_dir_all(&quickshare_dir)
            .map_err(|e| e.to_string())?;

        // 创建各个脚本文件
        let scripts = vec![
            ("Send to QuickShare", "--quickshare-send"),
            ("Send to Multiple Devices", "--quickshare-send-multi"),
            ("Send to Recent Device", "--quickshare-send-recent"),
            ("Add to Send Queue", "--quickshare-queue"),
        ];

        for (name, arg) in scripts {
            let script_path = quickshare_dir.join(name);
            let script_content = format!(
                "#!/bin/bash\n\"{}\" {} \"$@\"\n",
                app_path, arg
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
        }

        Ok(())
    }

    /// Linux: 卸载右键菜单
    #[cfg(target_os = "linux")]
    pub fn unregister_context_menu() -> Result<(), String> {
        use std::fs;

        let quickshare_dir = dirs::home_dir()
            .ok_or("Could not find home directory")?
            .join(".local/share/nautilus/scripts/QuickShare");

        if quickshare_dir.exists() {
            fs::remove_dir_all(&quickshare_dir)
                .map_err(|e| format!("Failed to remove QuickShare scripts: {}", e))?;
        }

        Ok(())
    }

    /// 未支持的平台
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn register_context_menu(_app_path: &str) -> Result<(), String> {
        Err("Context menu registration not supported on this platform".to_string())
    }

    /// 未支持的平台 - 卸载
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn unregister_context_menu() -> Result<(), String> {
        Err("Context menu unregistration not supported on this platform".to_string())
    }
}
