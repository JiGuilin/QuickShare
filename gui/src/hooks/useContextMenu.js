import { useState, useCallback } from "react";

/**
 * Hook 用于注册/卸载系统上下文菜单
 * 这应该只在设置页面调用一次
 */
export function useContextMenuRegistration() {
  const [isRegistering, setIsRegistering] = useState(false);
  const [isUnregistering, setIsUnregistering] = useState(false);
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
      setRegistrationStatus(result || "success");
    } catch (error) {
      console.error("[ContextMenu] Registration failed:", error);
      setRegistrationError(String(error));
    } finally {
      setIsRegistering(false);
    }
  }, []);

  const unregisterContextMenu = useCallback(async () => {
    if (!window.__TAURI__) {
      setRegistrationError("Context menu unregistration only available in Tauri");
      return;
    }

    setIsUnregistering(true);
    setRegistrationError("");
    setRegistrationStatus("");

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke("unregister_context_menu");
      setRegistrationStatus(result || "success");
    } catch (error) {
      console.error("[ContextMenu] Unregistration failed:", error);
      setRegistrationError(String(error));
    } finally {
      setIsUnregistering(false);
    }
  }, []);

  return {
    registerContextMenu,
    unregisterContextMenu,
    isRegistering,
    isUnregistering,
    registrationStatus,
    registrationError,
  };
}
