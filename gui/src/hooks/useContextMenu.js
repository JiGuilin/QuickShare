import { useState, useEffect, useCallback } from "react";

/**
 * Hook 用于监听和处理来自系统上下文菜单的文件
 * 支持以下事件：
 * 1. 从命令行参数接收的文件
 * 2. Tauri 事件监听器接收的文件
 */
export function useContextMenuFiles() {
  const [contextMenuFiles, setContextMenuFiles] = useState([]);
  const [isProcessing, setIsProcessing] = useState(false);

  useEffect(() => {
    if (!window.__TAURI__) return;

    let unlistenFn = null;
    let cancelled = false;

    const setupListener = async () => {
      try {
        // 1. 首先检查是否有命令行参数传入的文件
        const { invoke } = await import("@tauri-apps/api/core");
        const cliFiles = await invoke("get_cli_files");
        
        if (cancelled) return;
        
        if (cliFiles && cliFiles.length > 0) {
          console.log("[ContextMenu] Received CLI files:", cliFiles);
          await handleFilesReceived(cliFiles);
        }

        // 2. 监听 Tauri 事件以获取从上下文菜单传入的文件
        const eventMod = await import("@tauri-apps/api/event");
        unlistenFn = await eventMod.listen("quickshare://context-menu-files", (event) => {
          if (cancelled) return;
          
          const files = event.payload;
          console.log("[ContextMenu] Received context menu files:", files);
          handleFilesReceived(files);
        });
      } catch (error) {
        console.error("[ContextMenu] Setup failed:", error);
      }
    };

    const handleFilesReceived = async (files) => {
      if (!files || files.length === 0) return;
      
      setIsProcessing(true);
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        
        // 调用 Rust 后端来处理文件
        const fileObjects = await invoke("handle_context_menu_files", {
          files: files,
        });

        if (cancelled) return;

        console.log("[ContextMenu] Processed files:", fileObjects);
        setContextMenuFiles(fileObjects);
      } catch (error) {
        console.error("[ContextMenu] Failed to handle files:", error);
      } finally {
        setIsProcessing(false);
      }
    };

    setupListener();

    return () => {
      cancelled = true;
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, []);

  const clearContextMenuFiles = useCallback(() => {
    setContextMenuFiles([]);
  }, []);

  return {
    contextMenuFiles,
    isProcessing,
    clearContextMenuFiles,
  };
}

/**
 * Hook 用于注册系统上下文菜单
 * 这应该只在设置页面调用一次
 */
export function useContextMenuRegistration() {
  const [isRegistering, setIsRegistering] = useState(false);
  const [registrationStatus, setRegistrationStatus] = useState("");
  const [registrationError, setRegistrationError] = useState("");

  const registerContextMenu = useCallback(async () => {
    if (!window.__TAURI__) {
      setRegistrationError("Context menu registration only available in Tauri");
      return;
    }

    setIsRegistering(true);
    setRegistrationError("");
    setRegistrationStatus("");

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke("register_context_menu");
      setRegistrationStatus(result);
    } catch (error) {
      console.error("[ContextMenu] Registration failed:", error);
      setRegistrationError(String(error));
    } finally {
      setIsRegistering(false);
    }
  }, []);

  return {
    registerContextMenu,
    isRegistering,
    registrationStatus,
    registrationError,
  };
}
