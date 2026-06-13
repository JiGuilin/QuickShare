import { useState, useCallback } from "react";
import {
  Send,
  Download,
  Wifi,
  Settings,
  Monitor,
  Smartphone,
  FileText,
  Image,
  Film,
  Music,
  Archive,
  Check,
  ChevronRight,
  RefreshCw,
  FolderOpen,
  Zap,
  Globe,
} from "lucide-react";
import { useI18n, availableLocales } from "./i18n";

// ─── API helpers ─────────────────────────────────────────────
const API_BASE = "http://localhost:53318";

// ─── Icon helpers ────────────────────────────────────────────
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
  if (["jpg", "jpeg", "png", "gif", "webp", "svg"].includes(fileType.toLowerCase()))
    return <Image size={20} className="text-green-500" />;
  if (["mp4", "mkv", "avi", "mov"].includes(fileType.toLowerCase()))
    return <Film size={20} className="text-purple-500" />;
  if (["mp3", "wav", "flac", "aac"].includes(fileType.toLowerCase()))
    return <Music size={20} className="text-orange-500" />;
  if (["zip", "rar", "7z", "tar", "gz"].includes(fileType.toLowerCase()))
    return <Archive size={20} className="text-yellow-500" />;
  return <FileText size={20} className="text-gray-400" />;
}

function formatSize(bytes) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

// ─── Components ──────────────────────────────────────────────

function Sidebar({ activeTab, setActiveTab }) {
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
          <div className="w-2 h-2 bg-green-400 rounded-full"></div>
          {t("sidebar.online")}
        </div>
      </div>
    </aside>
  );
}

function ReceiveTab({ transfers }) {
  const { t } = useI18n();

  return (
    <div className="animate-fade-in">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-800">{t("receive.title")}</h2>
        <p className="text-sm text-gray-500 mt-1">{t("receive.subtitle")}</p>
      </div>

      {/* Status indicator */}
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

      {/* Transfer list */}
      {transfers.length === 0 ? (
        <div className="text-center py-12">
          <FolderOpen size={48} className="mx-auto text-gray-300 mb-3" />
          <p className="text-gray-400">{t("receive.noTransfers")}</p>
          <p className="text-xs text-gray-300 mt-1">{t("receive.noTransfersHint")}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {transfers.map((t) => (
            <TransferCard key={t.id} transfer={t} />
          ))}
        </div>
      )}
    </div>
  );
}

function TransferCard({ transfer }) {
  const { t } = useI18n();
  const progress = transfer.totalBytes
    ? Math.round((transfer.bytesTransferred / transfer.totalBytes) * 100)
    : 0;

  return (
    <div className="bg-white rounded-xl border border-gray-200 p-4 animate-slide-in">
      <div className="flex items-center gap-3 mb-3">
        <FileIcon fileType={transfer.files?.[0]?.fileType} />
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-gray-800 truncate">
            {transfer.files?.[0]?.name || "Unknown file"}
          </p>
          <p className="text-xs text-gray-400">
            {formatSize(transfer.bytesTransferred)} / {formatSize(transfer.totalBytes)}
          </p>
        </div>
        <span className="text-xs text-gray-400">{progress}%</span>
      </div>
      <div className="w-full bg-gray-100 rounded-full h-1.5">
        <div
          className="bg-primary-500 h-1.5 rounded-full transition-all duration-300"
          style={{ width: `${progress}%` }}
        ></div>
      </div>
      <div className="flex items-center justify-between mt-2">
        <span className="text-xs text-gray-400">{t("receive.from")}: {transfer.senderAlias || "Unknown"}</span>
        <span className="text-xs text-green-500">{transfer.status}</span>
      </div>
    </div>
  );
}

function SendTab({ devices }) {
  const { t } = useI18n();
  const [selectedFiles, setSelectedFiles] = useState([]);
  const [selectedDevice, setSelectedDevice] = useState(null);
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState(false);

  const handleSend = async () => {
    if (!selectedDevice || selectedFiles.length === 0) return;
    setSending(true);
    setTimeout(() => {
      setSending(false);
      setSent(true);
      setTimeout(() => setSent(false), 3000);
    }, 2000);
  };

  return (
    <div className="animate-fade-in">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-800">{t("send.title")}</h2>
        <p className="text-sm text-gray-500 mt-1">{t("send.subtitle")}</p>
      </div>

      {/* File selection */}
      <div className="bg-white rounded-xl border-2 border-dashed border-gray-200 p-8 mb-6 text-center hover:border-primary-300 transition-colors cursor-pointer">
        <FileText size={32} className="mx-auto text-gray-300 mb-2" />
        <p className="text-sm text-gray-500">{t("send.dragDrop")}</p>
        {selectedFiles.length > 0 && (
          <div className="mt-4 flex flex-wrap gap-2 justify-center">
            {selectedFiles.map((f, i) => (
              <span key={i} className="px-3 py-1 bg-primary-50 text-primary-600 rounded-full text-xs font-medium">
                {f}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Device selection */}
      <div className="mb-6">
        <h3 className="text-sm font-medium text-gray-700 mb-3">{t("send.selectTarget")}</h3>
        {devices.length === 0 ? (
          <div className="bg-gray-50 rounded-lg p-4 text-center">
            <p className="text-sm text-gray-400">{t("send.noDevices")}</p>
            <button className="mt-2 text-xs text-primary-500 hover:text-primary-600 flex items-center gap-1 mx-auto">
              <RefreshCw size={12} /> {t("send.scanAgain")}
            </button>
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

      {/* Send button */}
      <button
        onClick={handleSend}
        disabled={!selectedDevice || selectedFiles.length === 0 || sending}
        className={`w-full py-3 rounded-xl font-medium text-white transition-all ${
          sent
            ? "bg-green-500"
            : !selectedDevice || selectedFiles.length === 0
            ? "bg-gray-200 text-gray-400 cursor-not-allowed"
            : sending
            ? "bg-primary-400 cursor-wait"
            : "bg-primary-500 hover:bg-primary-600"
        }`}
      >
        {sent ? (
          <span className="flex items-center justify-center gap-2">
            <Check size={18} /> {t("send.sent")}
          </span>
        ) : sending ? (
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

function DevicesTab({ devices, scanning, onScan }) {
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
                  {device.ip} · {device.os || "Unknown OS"} · v{device.version}
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

function SettingsTab() {
  const { t, locale, setLocale } = useI18n();

  return (
    <div className="animate-fade-in">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-800">{t("settings.title")}</h2>
        <p className="text-sm text-gray-500 mt-1">{t("settings.subtitle")}</p>
      </div>

      <div className="space-y-4">
        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <label className="text-sm font-medium text-gray-700">{t("settings.deviceAlias")}</label>
          <input
            type="text"
            defaultValue="My Mac"
            className="mt-1 w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 focus:border-primary-300"
          />
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <label className="text-sm font-medium text-gray-700">{t("settings.port")}</label>
          <input
            type="number"
            defaultValue={53318}
            className="mt-1 w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 focus:border-primary-300"
          />
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <label className="text-sm font-medium text-gray-700">{t("settings.downloadDir")}</label>
          <div className="mt-1 flex gap-2">
            <input
              type="text"
              defaultValue="~/Downloads/QuickShare"
              className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 focus:border-primary-300"
              readOnly
            />
            <button className="px-3 py-2 bg-gray-100 rounded-lg text-sm hover:bg-gray-200 transition-colors">
              {t("settings.browse")}
            </button>
          </div>
        </div>

        {/* 语言设置 */}
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
            <button className="w-10 h-6 bg-primary-500 rounded-full relative">
              <div className="w-4 h-4 bg-white rounded-full absolute right-1 top-1"></div>
            </button>
          </div>
        </div>

        <div className="bg-white rounded-xl border border-gray-200 p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-700">{t("settings.startAtLogin")}</p>
              <p className="text-xs text-gray-400">{t("settings.startAtLoginHint")}</p>
            </div>
            <button className="w-10 h-6 bg-gray-200 rounded-full relative">
              <div className="w-4 h-4 bg-white rounded-full absolute left-1 top-1"></div>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── Main App ────────────────────────────────────────────────

export default function App() {
  const [activeTab, setActiveTab] = useState("receive");
  const [devices, setDevices] = useState([
    { id: "1", alias: "iPhone 15", ip: "192.168.1.105", port: 53318, deviceType: "mobile", os: "iOS", version: "1.0" },
    { id: "2", alias: "Windows PC", ip: "192.168.1.110", port: 53318, deviceType: "desktop", os: "Windows", version: "1.0" },
    { id: "3", alias: "MacBook Pro", ip: "192.168.1.120", port: 53318, deviceType: "desktop", os: "macOS", version: "1.0" },
  ]);
  const [transfers, setTransfers] = useState([]);
  const [scanning, setScanning] = useState(false);

  const handleScan = useCallback(() => {
    setScanning(true);
    setTimeout(() => setScanning(false), 3000);
  }, []);

  return (
    <div className="flex h-screen bg-gray-50">
      <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} />
      <main className="flex-1 p-6 overflow-y-auto">
        {activeTab === "receive" && <ReceiveTab transfers={transfers} />}
        {activeTab === "send" && <SendTab devices={devices} />}
        {activeTab === "devices" && (
          <DevicesTab devices={devices} scanning={scanning} onScan={handleScan} />
        )}
        {activeTab === "settings" && <SettingsTab />}
      </main>
    </div>
  );
}
