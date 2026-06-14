import { useState, useCallback, useRef, useEffect } from "react";
import {
  Send,
  Download,
  Wifi,
  WifiOff,
  Settings,
  Monitor,
  Smartphone,
  FileText,
  Image,
  Film,
  Music,
  Archive,
  Check,
  X,
  ChevronRight,
  RefreshCw,
  FolderOpen,
  Zap,
  Globe,
  Upload,
  Clock,
  AlertCircle,
  Folder,
} from "lucide-react";
import { useI18n, availableLocales } from "./i18n";
import { useQuickShare } from "./hooks/useWebSocket";

// Tauri plugins - imported statically, but only used when window.__TAURI__ is available
import * as tauriDialog from "@tauri-apps/plugin-dialog";
import * as tauriAutostart from "@tauri-apps/plugin-autostart";

const API_BASE = "http://localhost:53318";

function DeviceIcon({ type }) {
  switch (type) {
    case "mobile":
      return <Smartphone size={20} className="text-blue-500" />;
    case "desktop":
      return <Monitor size={20} className="text-blue-500" />;
    default:
      return <Monitor size={20} className="text-gray-400" />;
  }
}

function FileIcon({ fileType }) {
  if (!fileType) return <FileText size={20} />;
  const ft = fileType.toLowerCase();
  if (["jpg", "jpeg", "png", "gif", "webp", "svg", "image/"].some((x) => ft.includes(x)))
    return <Image size={20} className="text-green-500" />;
  if (["mp4", "mkv", "avi", "mov", "video/"].some((x) => ft.includes(x)))
    return <Film size={20} className="text-purple-500" />;
  if (["mp3", "wav", "flac", "aac", "audio/"].some((x) => ft.includes(x)))
    return <Music size={20} className="text-orange-500" />;
  if (["zip", "rar", "7z", "tar", "gz", "application/zip", "application/x-"].some((x) => ft.includes(x)))
    return <Archive size={20} className="text-yellow-500" />;
  return <FileText size={20} className="text-gray-400" />;
}

function formatSize(bytes) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

function formatSpeed(bps) {
  if (!bps || bps <= 0) return "";
  if (bps >= 1024 * 1024) return `${(bps / 1024 / 1024).toFixed(1)} MB/s`;
  if (bps >= 1024) return `${(bps / 1024).toFixed(0)} KB/s`;
  return `${bps} B/s`;
}

// ─── Components ──────────────────────────────────────────────

function Sidebar({ activeTab, setActiveTab, connected }) {
  const { t } = useI18n();
  const tabs = [
    { id: "receive", label: t("sidebar.receive"), icon: Download },
    { id: "send", label: t("sidebar.send"), icon: Send },
    { id: "devices", label: t("sidebar.devices"), icon: Wifi },
    { id: "settings", label: t("sidebar.settings"), icon: Settings },
  ];

  return (
    <aside className="w-56 bg-white border-r border-gray-200 flex flex-col">
      <div className="p-5 border-b border-gray-100">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center">
            <Zap size={18} className="text-white" />
          </div>
          <span className="text-lg font-bold text-gray-800">{t("common.appname")}</span>
        </div>
      </div>
      <nav className="flex-1 p-3 space-y-1">
        {tabs.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id)}
            className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all ${
              activeTab === id
                ? "bg-primary-50 text-primary-600"
                : "text-gray-600 hover:bg-gray-50 hover:text-gray-900"
            }`}
          >
            <Icon size={18} />
            {label}
          </button>
        ))}
      </nav>
      <div className="p-4 border-t border-gray-100">
        <div className="flex items-center gap-2 text-xs text-gray-400">
          {connected ? (
            <>
              <div className="w-2 h-2 bg-green-400 rounded-full"></div>
              {t("sidebar.online")}
            </>
          ) : (
            <>
              <div className="w-2 h-2 bg-red-400 rounded-full"></div>
              {t("sidebar.offline") || "Offline"}
            </>
          )}
        </div>
      </div>
    </aside>
  );
}

function ReceiveTab({ transfers, onAccept, onReject }) {
  const { t } = useI18n();

  // Only show INCOMING transfers in the receive tab
  const incomingOnly = transfers.filter((tr) => tr.direction === "incoming");
  const incomingTransfers = incomingOnly.filter(
    (tr) => tr.status === "pending" || tr.status === "receiving" || tr.status === "transferring" || tr.status === "waiting_accept"
  );
  const completedTransfers = incomingOnly.filter(
    (tr) => tr.status === "completed" || tr.status === "rejected" || tr.status === "error"
  );

  return (
    <div className="animate-fade-in">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-800">{t("receive.title")}</h2>
        <p className="text-sm text-gray-500 mt-1">{t("receive.subtitle")}</p>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <div className="flex items-center gap-4">
          <div className="relative">
            <div className="w-12 h-12 bg-green-100 rounded-full flex items-center justify-center">
              <Wifi size={24} className="text-green-500" />
            </div>
            <div className="absolute inset-0 w-12 h-12 bg-green-200 rounded-full animate-pulse-ring"></div>
          </div>
          <div>
            <p className="font-medium text-gray-800">{t("receive.listening")}</p>
            <p className="text-sm text-gray-500">{t("receive.port")}: 53318 · {t("receive.visible")}</p>
          </div>
        </div>
      </div>

      {incomingTransfers.length === 0 && completedTransfers.length === 0 ? (
        <div className="text-center py-12">
          <FolderOpen size={48} className="mx-auto text-gray-300 mb-3" />
          <p className="text-gray-400">{t("receive.noTransfers")}</p>
          <p className="text-xs text-gray-300 mt-1">{t("receive.noTransfersHint")}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {incomingTransfers.map((tr) => (
            <IncomingTransferCard
              key={tr.id}
              transfer={tr}
              onAccept={onAccept}
              onReject={onReject}
            />
          ))}
          {completedTransfers.map((tr) => (
            <CompletedTransferCard key={tr.id} transfer={tr} />
          ))}
        </div>
      )}
    </div>
  );
}

function IncomingTransferCard({ transfer, onAccept, onReject }) {
  const { t } = useI18n();
  const progress = transfer.totalBytes
    ? Math.round((transfer.bytesTransferred / transfer.totalBytes) * 100)
    : 0;

  return (
    <div className="bg-white rounded-xl border border-gray-200 p-4 animate-slide-in">
      <div className="flex items-center gap-3 mb-3">
        <FileIcon fileType={transfer.files?.[0]?.file_type || transfer.files?.[0]?.fileType} />
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-gray-800 truncate">
            {transfer.files?.length === 1
              ? transfer.files[0].name
              : `${transfer.files?.length} ${t("send.files") || "files"}`}
          </p>
          <p className="text-xs text-gray-400">
            {formatSize(transfer.bytesTransferred)} / {formatSize(transfer.totalBytes)}
            {transfer.from && ` · ${t("receive.from")}: ${transfer.from.alias}`}
          </p>
        </div>
      </div>

      {transfer.status === "pending" ? (
        <div className="flex gap-2 mt-3">
          <button
            onClick={() => onAccept(transfer)}
            className="flex-1 py-2 bg-primary-500 text-white rounded-lg text-sm font-medium hover:bg-primary-600 transition-colors flex items-center justify-center gap-1"
          >
            <Check size={14} /> {t("receive.accept") || "Accept"}
          </button>
          <button
            onClick={() => onReject(transfer)}
            className="flex-1 py-2 bg-gray-100 text-gray-600 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors flex items-center justify-center gap-1"
          >
            <X size={14} /> {t("receive.reject") || "Reject"}
          </button>
        </div>
      ) : transfer.status === "waiting_accept" ? (
        <div className="flex items-center gap-2 mt-3 text-xs text-amber-600">
          <Clock size={14} />
          <span>{t("receive.waitingAccept") || "Waiting for receiver to accept..."}</span>
        </div>
      ) : (
        <>
          <div className="w-full bg-gray-100 rounded-full h-1.5 mt-2">
            <div
              className="bg-primary-500 h-1.5 rounded-full transition-all duration-300"
              style={{ width: `${progress}%` }}
            ></div>
          </div>
          <div className="flex items-center justify-between mt-2">
            <span className="text-xs text-gray-400">{progress}% · {formatSize(transfer.bytesTransferred)} / {formatSize(transfer.totalBytes)}</span>
            <span className="text-xs text-primary-500">
              {transfer.status === "receiving" ? (t("receive.receiving") || "Receiving...") : (t("receive.transferring") || "Transferring...")}
              {transfer.speed ? ` · ${formatSpeed(transfer.speed)}` : ""}
            </span>
          </div>
        </>
      )}
    </div>
  );
}

function CompletedTransferCard({ transfer }) {
  const { t } = useI18n();
  return (
    <div className="bg-white rounded-xl border border-gray-100 p-4 opacity-60">
      <div className="flex items-center gap-3">
        {transfer.status === "completed" ? (
          <Check size={20} className="text-green-500" />
        ) : transfer.status === "error" ? (
          <AlertCircle size={20} className="text-red-400" />
        ) : (
          <X size={20} className="text-red-400" />
        )}
        <div className="flex-1 min-w-0">
          <p className="text-sm text-gray-600 truncate">
            {transfer.files?.[0]?.name || "Transfer"}
          </p>
        </div>
        <span className={`text-xs ${
          transfer.status === "completed" ? "text-green-500" : transfer.status === "error" ? "text-red-400" : "text-red-400"
        }`}>
          {transfer.status === "completed" ? (t("receive.completed") || "Completed")
            : transfer.status === "error" ? (t("receive.error") || "Error")
            : (t("receive.rejected") || "Rejected")}
        </span>
      </div>
    </div>
  );
}

function SendTab({ devices, onSend, myDevice, transfers }) {
  const { t } = useI18n();
  const [selectedFiles, setSelectedFiles] = useState([]);
  const [fileObjects, setFileObjects] = useState([]);
  const [selectedDevice, setSelectedDevice] = useState(null);
  const [dragOver, setDragOver] = useState(false);
  const fileInputRef = useRef(null);

  // Get outgoing transfers for this tab
  const outgoingTransfers = transfers.filter((tr) => tr.direction === "outgoing");
  const activeOutgoing = outgoingTransfers.filter(
    (tr) => tr.status !== "completed" && tr.status !== "rejected" && tr.status !== "error"
  );
  const completedOutgoing = outgoingTransfers.filter(
    (tr) => tr.status === "completed" || tr.status === "error"
  );
  const sending = activeOutgoing.length > 0;

  // Listen for Tauri drag-drop events
  useEffect(() => {
    if (!window.__TAURI__) return;

    let unlistenDrop, unlistenEnter, unlistenLeave;

    const setup = async () => {
      // Dynamic import Tauri APIs inside effect to ensure availability
      const [eventMod, coreMod] = await Promise.all([
        import("@tauri-apps/api/event"),
        import("@tauri-apps/api/core"),
      ]);
      const listen = eventMod.listen;
      const convertFileSrc = coreMod.convertFileSrc;

      // Helper to read a local file path into a File object
      const readPath = async (filePath) => {
        try {
          const assetUrl = convertFileSrc(filePath);
          const response = await fetch(assetUrl);
          if (response.ok) {
            const blob = await response.blob();
            const name = filePath.split(/[/\\]/).pop();
            return new File([blob], name, { type: blob.type || "application/octet-stream" });
          }
        } catch (e) {
          console.warn("Failed to read file via asset protocol:", e);
        }
        return null;
      };

      const handleDropped = async (paths) => {
        const files = (await Promise.all(paths.map(readPath))).filter(Boolean);
        if (files.length > 0) {
          setFileObjects((prev) => [...prev, ...files]);
          setSelectedFiles((prev) => [...prev, ...files.map((f) => f.name)]);
        }
      };

      unlistenDrop = await listen("tauri://drag-drop", (event) => {
        setDragOver(false);
        const paths = event.payload?.paths || [];
        if (paths.length > 0) {
          handleDropped(paths);
        }
      });

      unlistenEnter = await listen("tauri://drag-enter", () => {
        setDragOver(true);
      });

      unlistenLeave = await listen("tauri://drag-leave", () => {
        setDragOver(false);
      });
    };

    setup();

    return () => {
      if (unlistenDrop) unlistenDrop();
      if (unlistenEnter) unlistenEnter();
      if (unlistenLeave) unlistenLeave();
    };
  }, []);

  const handleFileSelect = (e) => {
    const files = Array.from(e.target.files || []);
    setFileObjects(files);
    setSelectedFiles(files.map((f) => f.name));
  };

  const handleDrop = (e) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
    // In Tauri, the drag-drop is handled by Tauri events above
    // This handler works for browser (non-Tauri) mode
    if (!window.__TAURI__) {
      const files = Array.from(e.dataTransfer.files || []);
      setFileObjects(files);
      setSelectedFiles(files.map((f) => f.name));
    }
  };

  const handleDragOver = (e) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(true);
  };

  const handleDragLeave = (e) => {
    e.preventDefault();
    setDragOver(false);
  };

  const handleSend = async () => {
    if (!selectedDevice || fileObjects.length === 0) return;
    try {
      await onSend(selectedDevice, fileObjects);
      setSelectedFiles([]);
      setFileObjects([]);
      setSelectedDevice(null);
    } catch (err) {
      console.error("Send error:", err);
    }
  };

  return (
    <div className="animate-fade-in">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-800">{t("send.title")}</h2>
        <p className="text-sm text-gray-500 mt-1">{t("send.subtitle")}</p>
      </div>

      {/* Outgoing transfer progress */}
      {(activeOutgoing.length > 0 || completedOutgoing.length > 0) && (
        <div className="mb-6 space-y-3">
          {activeOutgoing.map((tr) => {
            const progress = tr.totalBytes ? Math.round((tr.bytesTransferred / tr.totalBytes) * 100) : 0;
            return (
              <div key={tr.id} className="bg-white rounded-xl border border-gray-200 p-4 animate-slide-in">
                <div className="flex items-center gap-3 mb-2">
                  <Send size={16} className="text-primary-500" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-800 truncate">
                      {tr.files?.length === 1
                        ? tr.files[0].name
                        : `${tr.files?.length} ${t("send.files") || "files"}`}
                    </p>
                    <p className="text-xs text-gray-400">
                      {t("send.to") || "To"}: {tr.targetDevice?.alias || "Unknown"}
                      {tr.status === "waiting_accept" && ` · ${t("receive.waitingAccept") || "Waiting..."}`}
                    </p>
                  </div>
                </div>
                {tr.status !== "waiting_accept" && tr.status !== "preparing" && (
                  <>
                    <div className="w-full bg-gray-100 rounded-full h-1.5">
                      <div
                        className="bg-primary-500 h-1.5 rounded-full transition-all duration-300"
                        style={{ width: `${progress}%` }}
                      ></div>
                    </div>
                    <div className="flex items-center justify-between mt-1">
                      <span className="text-xs text-gray-400">{progress}% · {formatSize(tr.bytesTransferred)} / {formatSize(tr.totalBytes)}</span>
                      <span className="text-xs text-primary-500">
                        {tr.status === "transferring" ? (t("send.sending") || "Sending...") : tr.status}
                        {tr.speed ? ` · ${formatSpeed(tr.speed)}` : ""}
                      </span>
                    </div>
                  </>
                )}
              </div>
            );
          })}
          {completedOutgoing.slice(-3).map((tr) => (
            <div key={tr.id} className="bg-white rounded-xl border border-gray-100 p-3 opacity-60">
              <div className="flex items-center gap-2">
                {tr.status === "completed" ? (
                  <Check size={16} className="text-green-500" />
                ) : (
                  <AlertCircle size={16} className="text-red-400" />
                )}
                <p className="text-xs text-gray-500 truncate">
                  {tr.files?.[0]?.name || "Transfer"} — {tr.status === "completed" ? (t("receive.completed") || "Completed") : (t("receive.error") || "Error")}
                </p>
              </div>
            </div>
          ))}
        </div>
      )}

      <div
        className={`bg-white rounded-xl border-2 border-dashed p-8 mb-6 text-center transition-colors cursor-pointer ${
          dragOver
            ? "border-primary-400 bg-primary-50"
            : "border-gray-200 hover:border-primary-300"
        }`}
        onClick={() => fileInputRef.current?.click()}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          className="hidden"
          onChange={handleFileSelect}
        />
        <Upload size={32} className="mx-auto text-gray-300 mb-2" />
        <p className="text-sm text-gray-500">{t("send.dragDrop")}</p>
        {selectedFiles.length > 0 && (
          <div className="mt-4 flex flex-col gap-1 max-h-40 overflow-y-auto">
            {selectedFiles.map((f, i) => (
              <div key={i} className="flex items-center gap-2 px-3 py-1 bg-primary-50 text-primary-600 rounded-lg text-xs font-medium mx-auto">
                <FileIcon fileType={f.split('.').pop()} />
                <span>{f}</span>
                <span className="text-gray-400">
                  {fileObjects[i] ? formatSize(fileObjects[i].size) : ""}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="mb-6">
        <h3 className="text-sm font-medium text-gray-700 mb-3">{t("send.selectTarget")}</h3>
        {devices.length === 0 ? (
          <div className="bg-gray-50 rounded-lg p-4 text-center">
            <Wifi size={24} className="mx-auto text-gray-300 mb-2" />
            <p className="text-sm text-gray-400">{t("send.noDevices")}</p>
            <p className="text-xs text-gray-300 mt-1">{t("devices.noDevicesHint")}</p>
          </div>
        ) : (
          <div className="space-y-2">
            {devices.map((device) => (
              <button
                key={device.id}
                onClick={() => setSelectedDevice(device)}
                className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-all ${
                  selectedDevice?.id === device.id
                    ? "border-primary-300 bg-primary-50"
                    : "border-gray-200 bg-white hover:border-gray-300"
                }`}
              >
                <DeviceIcon type={device.deviceType} />
                <div className="flex-1 text-left">
                  <p className="text-sm font-medium text-gray-800">{device.alias}</p>
                  <p className="text-xs text-gray-400">{device.ip}:{device.port}</p>
                </div>
                {selectedDevice?.id === device.id && (
                  <Check size={16} className="text-primary-500" />
                )}
                <ChevronRight size={16} className="text-gray-300" />
              </button>
            ))}
          </div>
        )}
      </div>

      <button
        onClick={handleSend}
        disabled={!selectedDevice || fileObjects.length === 0 || sending}
        className={`w-full py-3 rounded-xl font-medium text-white transition-all ${
          !selectedDevice || fileObjects.length === 0
          ? "bg-gray-200 text-gray-400 cursor-not-allowed"
          : sending
          ? "bg-primary-400 cursor-wait"
          : "bg-primary-500 hover:bg-primary-600"
        }`}
      >
        {sending ? (
          <span className="flex items-center justify-center gap-2">
            <RefreshCw size={18} className="animate-spin" /> {t("send.sending")}
          </span>
        ) : (
          <span className="flex items-center justify-center gap-2">
            <Send size={18} /> {t("send.sendFiles")}
          </span>
        )}
      </button>
    </div>
  );
}

function DevicesTab({ devices, scanning, onScan, connected }) {
  const { t, tc } = useI18n();

  return (
    <div className="animate-fade-in">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-xl font-semibold text-gray-800">{t("devices.title")}</h2>
          <p className="text-sm text-gray-500 mt-1">
            {tc("devices.subtitle", { count: devices.length })}
          </p>
        </div>
        <button
          onClick={onScan}
          disabled={scanning}
          className="flex items-center gap-2 px-4 py-2 bg-primary-50 text-primary-600 rounded-lg text-sm font-medium hover:bg-primary-100 transition-colors"
        >
          <RefreshCw size={14} className={scanning ? "animate-spin" : ""} />
          {scanning ? t("devices.scanning") : t("devices.scan")}
        </button>
      </div>

      <div className={`mb-4 p-3 rounded-lg flex items-center gap-2 text-sm ${
        connected ? "bg-green-50 text-green-700" : "bg-red-50 text-red-600"
      }`}>
        {connected ? <Wifi size={16} /> : <WifiOff size={16} />}
        {connected ? (t("devices.connected") || "Connected to server") : (t("devices.disconnected") || "Disconnected from server")}
      </div>

      {devices.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
          <Wifi size={48} className="mx-auto text-gray-300 mb-3" />
          <p className="text-gray-400">
            {scanning ? t("devices.scanning") : t("devices.noDevices")}
          </p>
          <p className="text-xs text-gray-300 mt-1">
            {t("devices.noDevicesHint")}
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {devices.map((device, i) => (
            <div
              key={device.id}
              className="bg-white rounded-xl border border-gray-200 p-4 flex items-center gap-4 animate-slide-in"
              style={{ animationDelay: `${i * 0.05}s` }}
            >
              <div className="w-10 h-10 bg-blue-50 rounded-full flex items-center justify-center">
                <DeviceIcon type={device.deviceType} />
              </div>
              <div className="flex-1">
                <p className="font-medium text-gray-800">{device.alias}</p>
                <p className="text-xs text-gray-400">
                  {device.ip}:{device.port} · {device.os || "Unknown OS"} · v{device.version}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-green-400 rounded-full"></div>
                <span className="text-xs text-gray-400">{t("devices.online")}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SettingsTab({ settings, onUpdateSettings }) {
  const { t, locale, setLocale } = useI18n();
  const [alias, setAlias] = useState(settings.alias);
  const [downloadDir, setDownloadDir] = useState(settings.download_dir);
  const [autoAccept, setAutoAccept] = useState(settings.auto_accept);
  const [startAtLogin, setStartAtLogin] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  // Sync from props when settings update
  useEffect(() => {
    setAlias(settings.alias);
    setDownloadDir(settings.download_dir);
    setAutoAccept(settings.auto_accept);
  }, [settings.alias, settings.download_dir, settings.auto_accept]);

  // Check autostart status on mount
  useEffect(() => {
    if (window.__TAURI__) {
      tauriAutostart.isEnabled().then((enabled) => setStartAtLogin(enabled)).catch(() => {});
    }
  }, []);

  const handleSave = async () => {
    setSaving(true);
    await onUpdateSettings({
      alias: alias !== settings.alias ? alias : undefined,
      download_dir: downloadDir !== settings.download_dir ? downloadDir : undefined,
      auto_accept: autoAccept !== settings.auto_accept ? autoAccept : undefined,
    });
    setSaving(false);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const toggleAutoAccept = async () => {
    const newVal = !autoAccept;
    setAutoAccept(newVal);
    await onUpdateSettings({ auto_accept: newVal });
  };

  const toggleStartAtLogin = async () => {
    if (!window.__TAURI__) return;
    try {
      if (startAtLogin) {
        await tauriAutostart.disable();
        setStartAtLogin(false);
      } else {
        await tauriAutostart.enable();
        setStartAtLogin(true);
      }
    } catch (err) {
      console.error("Autostart toggle failed:", err);
    }
  };

  const browseDirectory = async () => {
    if (!window.__TAURI__) return;
    try {
      const selected = await tauriDialog.open({
        directory: true,
        multiple: false,
        title: t("settings.selectDownloadDir") || "Select Download Directory",
      });
      if (selected) {
        setDownloadDir(selected);
      }
    } catch (err) {
      console.error("Directory picker failed:", err);
    }
  };

  const generateRandomAlias = async () => {
    try {
      const locale = navigator.language.startsWith("zh") ? "zh" : "en";
      const resp = await fetch(`${API_BASE}/api/random-alias?locale=${locale}`);
      const data = await resp.json();
      if (data.alias) {
        setAlias(data.alias);
        // Save immediately so other devices see the change
        await onUpdateSettings({ alias: data.alias });
      }
    } catch (err) {
      console.error("Failed to generate random alias:", err);
    }
  };

  const useSystemName = async () => {
    try {
      // Use the backend API to get the system hostname (works in both Tauri and browser)
      const resp = await fetch(`${API_BASE}/api/info`);
      const data = await resp.json();
      // The device_model field contains the hostname
      if (data.device?.device_model) {
        setAlias(data.device.device_model);
        // Save immediately so other devices see the change
        await onUpdateSettings({ alias: data.device.device_model });
        return;
      }
    } catch (err) {
      // Backend not available yet
    }

    // Fallback
    setAlias("QuickShare");
  };

  return (
    <div className="animate-fade-in">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-800">{t("settings.title")}</h2>
        <p className="text-sm text-gray-500 mt-1">{t("settings.subtitle")}</p>
      </div>

      <div className="space-y-4">
        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <label className="text-sm font-medium text-gray-700">{t("settings.deviceAlias")}</label>
          <div className="mt-1 flex gap-2">
            <input
              type="text"
              value={alias || ""}
              onChange={(e) => setAlias(e.target.value)}
              className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 focus:border-primary-300"
            />
            <button
              onClick={generateRandomAlias}
              className="px-3 py-2 bg-primary-50 hover:bg-primary-100 text-primary-600 rounded-lg text-sm font-medium transition-colors flex items-center gap-1"
              title={t("settings.generateRandomAlias") || "Generate Random Alias"}
            >
              <Zap size={14} />
              {t("settings.generateRandomAlias") || "Random"}
            </button>
            <button
              onClick={useSystemName}
              className="px-3 py-2 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded-lg text-sm font-medium transition-colors"
              title={t("settings.useSystemName") || "Use System Name"}
            >
              <Monitor size={14} />
            </button>
          </div>
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <label className="text-sm font-medium text-gray-700">{t("settings.port")}</label>
          <input
            type="number"
            value={settings.port}
            disabled
            className="mt-1 w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-gray-50 text-gray-500"
          />
          <p className="text-xs text-gray-400 mt-1">{t("settings.portHint") || "Port change requires restart"}</p>
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <label className="text-sm font-medium text-gray-700">{t("settings.downloadDir")}</label>
          <div className="mt-1 flex gap-2">
            <input
              type="text"
              value={downloadDir || ""}
              onChange={(e) => setDownloadDir(e.target.value)}
              className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 focus:border-primary-300"
            />
            {window.__TAURI__ && (
              <button
                onClick={browseDirectory}
                className="px-3 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm font-medium text-gray-600 transition-colors flex items-center gap-1"
              >
                <Folder size={14} />
                {t("settings.browse") || "Browse"}
              </button>
            )}
          </div>
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <div className="flex items-center gap-2 mb-3">
            <Globe size={16} className="text-gray-500" />
            <label className="text-sm font-medium text-gray-700">{t("settings.language")}</label>
          </div>
          <div className="flex gap-2">
            {availableLocales.map(({ code, label }) => (
              <button
                key={code}
                onClick={() => setLocale(code)}
                className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                  locale === code
                    ? "bg-primary-500 text-white"
                    : "bg-gray-100 text-gray-600 hover:bg-gray-200"
                }`}
              >
                {code === "zh" ? "🇨🇳" : "🇺🇸"} {label}
              </button>
            ))}
          </div>
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-700">{t("settings.autoAccept")}</p>
              <p className="text-xs text-gray-400">{t("settings.autoAcceptHint")}</p>
            </div>
            <button
              onClick={toggleAutoAccept}
              className={`w-10 h-6 rounded-full relative transition-colors ${
                autoAccept ? "bg-primary-500" : "bg-gray-200"
              }`}
            >
              <div className={`w-4 h-4 bg-white rounded-full absolute top-1 transition-all ${
                autoAccept ? "right-1" : "left-1"
              }`}></div>
            </button>
          </div>
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-700">{t("settings.startAtLogin")}</p>
              <p className="text-xs text-gray-400">{t("settings.startAtLoginHint")}</p>
            </div>
            <button
              onClick={toggleStartAtLogin}
              className={`w-10 h-6 rounded-full relative transition-colors ${
                startAtLogin ? "bg-primary-500" : "bg-gray-200"
              }`}
            >
              <div className={`w-4 h-4 bg-white rounded-full absolute top-1 transition-all ${
                startAtLogin ? "right-1" : "left-1"
              }`}></div>
            </button>
          </div>
        </div>

        {/* Save button */}
        <button
          onClick={handleSave}
          disabled={saving}
          className={`w-full py-3 rounded-xl font-medium text-white transition-all ${
            saved
              ? "bg-green-500"
              : saving
              ? "bg-primary-400 cursor-wait"
              : "bg-primary-500 hover:bg-primary-600"
          }`}
        >
          {saved ? (
            <span className="flex items-center justify-center gap-2">
              <Check size={18} /> {t("settings.saved") || "Saved!"}
            </span>
          ) : saving ? (
            <span className="flex items-center justify-center gap-2">
              <RefreshCw size={18} className="animate-spin" /> {t("settings.saving") || "Saving..."}
            </span>
          ) : (
            <span className="flex items-center justify-center gap-2">
              {t("settings.save") || "Save Settings"}
            </span>
          )}
        </button>
      </div>
    </div>
  );
}

// ─── Main App ────────────────────────────────────────────────

export default function App() {
  const [activeTab, setActiveTab] = useState("receive");
  const [scanning, setScanning] = useState(false);
  const {
    devices, transfers, connected, settings, myDevice,
    sendFiles, acceptTransfer, rejectTransfer,
    scan, updateSettings,
  } = useQuickShare();

  // Filter out local device from the device list
  const otherDevices = myDevice
    ? devices.filter((d) => d.id !== myDevice.id)
    : devices;

  const handleScan = useCallback(async () => {
    setScanning(true);
    await scan();
    // Give more time for multicast responses to arrive
    setTimeout(() => setScanning(false), 3000);
  }, [scan]);

  return (
    <div className="flex h-screen bg-gray-50">
      <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} connected={connected} />
      <main className="flex-1 p-6 overflow-y-auto">
        {activeTab === "receive" && (
          <ReceiveTab transfers={transfers} onAccept={acceptTransfer} onReject={rejectTransfer} />
        )}
        {activeTab === "send" && (
          <SendTab devices={otherDevices} onSend={sendFiles} myDevice={myDevice} transfers={transfers} />
        )}
        {activeTab === "devices" && (
          <DevicesTab devices={otherDevices} scanning={scanning} onScan={handleScan} connected={connected} />
        )}
        {activeTab === "settings" && (
          <SettingsTab settings={settings} onUpdateSettings={updateSettings} />
        )}
      </main>
    </div>
  );
}
