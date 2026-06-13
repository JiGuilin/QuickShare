import { useState, useEffect, useRef, useCallback } from "react";

const WS_URL = "ws://localhost:53318/api/ws";
const API_BASE = "http://localhost:53318";

export function useQuickShare() {
  const [devices, setDevices] = useState([]);
  const [transfers, setTransfers] = useState([]);
  const [connected, setConnected] = useState(false);
  const [myDevice, setMyDevice] = useState(null);
  const [settings, setSettings] = useState({
    alias: "",
    port: 53318,
    download_dir: "",
    auto_accept: false,
  });
  const wsRef = useRef(null);
  const reconnectTimer = useRef(null);

  // ── connect ────────────────────────────────────────────
  const connect = useCallback(() => {
    if (wsRef.current && wsRef.current.readyState <= 1) return;

    const ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      setConnected(true);
    };

    ws.onclose = () => {
      setConnected(false);
      wsRef.current = null;
      reconnectTimer.current = setTimeout(() => connect(), 2000);
    };

    ws.onerror = () => {
      ws.close();
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        handleMessage(msg);
      } catch (e) {
        console.error("WS parse error:", e);
      }
    };

    wsRef.current = ws;
  }, []);

  // ── message handler ───────────────────────────────────
  const handleMessage = useCallback((msg) => {
    switch (msg.type) {
      case "hello": {
        setMyDevice(msg.device);
        if (msg.peers && msg.peers.length > 0) {
          setDevices((prev) => {
            const map = new Map(prev.map((d) => [d.id, d]));
            for (const p of msg.peers) map.set(p.id, p);
            return Array.from(map.values());
          });
        }
        break;
      }
      case "join": {
        setDevices((prev) => {
          const map = new Map(prev.map((d) => [d.id, d]));
          map.set(msg.device.id, msg.device);
          return Array.from(map.values());
        });
        break;
      }
      case "leave": {
        setDevices((prev) => prev.filter((d) => d.id !== msg.device_id));
        break;
      }
      case "transfer_request": {
        // Incoming transfer request from another device
        setTransfers((prev) => [
          ...prev,
          {
            id: msg.session_id || crypto.randomUUID(),
            sessionId: msg.session_id,
            from: msg.from,
            files: msg.files,
            status: "pending",
            bytesTransferred: 0,
            totalBytes: msg.files.reduce((s, f) => s + f.size, 0),
          },
        ]);
        break;
      }
      case "transfer_response": {
        setTransfers((prev) =>
          prev.map((t) =>
            t.sessionId === msg.session_id
              ? { ...t, status: msg.accepted ? "accepted" : "rejected" }
              : t
          )
        );
        break;
      }
      case "progress": {
        const p = msg.progress;
        setTransfers((prev) =>
          prev.map((t) =>
            t.sessionId === p.session_id
              ? {
                  ...t,
                  bytesTransferred: p.bytes_sent,
                  totalBytes: p.total_bytes,
                  status: "transferring",
                }
              : t
          )
        );
        break;
      }
      case "transfer_complete": {
        setTransfers((prev) =>
          prev.map((t) =>
            t.sessionId === msg.session_id
              ? { ...t, status: "completed", bytesTransferred: t.totalBytes }
              : t
          )
        );
        break;
      }
      default:
        break;
    }
  }, []);

  // ── initial connection ─────────────────────────────────
  useEffect(() => {
    // Fetch devices via REST as fallback
    fetch(`${API_BASE}/api/devices`)
      .then((r) => r.json())
      .then((list) => {
        if (Array.isArray(list) && list.length > 0) {
          setDevices(list);
        }
      })
      .catch(() => {});

    // Fetch settings
    fetch(`${API_BASE}/api/settings`)
      .then((r) => r.json())
      .then((s) => setSettings(s))
      .catch(() => {});

    connect();
    return () => {
      clearTimeout(reconnectTimer.current);
      if (wsRef.current) wsRef.current.close();
    };
  }, [connect]);

  // ── accept incoming transfer ───────────────────────────
  const acceptTransfer = useCallback(async (transfer) => {
    try {
      await fetch(`${API_BASE}/api/accept`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: transfer.sessionId }),
      });
      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transfer.id ? { ...t, status: "receiving" } : t
        )
      );
    } catch (err) {
      console.error("Accept failed:", err);
    }
  }, []);

  // ── reject incoming transfer ───────────────────────────
  const rejectTransfer = useCallback(async (transfer) => {
    try {
      await fetch(`${API_BASE}/api/reject`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: transfer.sessionId, reason: "User rejected" }),
      });
      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transfer.id ? { ...t, status: "rejected" } : t
        )
      );
    } catch (err) {
      console.error("Reject failed:", err);
    }
  }, []);

  // ── send files to a device ─────────────────────────────
  const sendFiles = useCallback(async (device, files) => {
    if (!device || !files || files.length === 0) return;

    const transferId = crypto.randomUUID();
    const totalSize = Array.from(files).reduce((s, f) => s + f.size, 0);

    setTransfers((prev) => [
      ...prev,
      {
        id: transferId,
        sessionId: null,
        from: myDevice,
        files: Array.from(files).map((f) => ({
          id: crypto.randomUUID(),
          name: f.name,
          size: f.size,
          file_type: f.type || "application/octet-stream",
        })),
        status: "preparing",
        bytesTransferred: 0,
        totalBytes: totalSize,
        targetDevice: device,
      },
    ]);

    try {
      const base = `http://${device.ip}:${device.port}`;

      // 1. Prepare send
      const fileMetas = Array.from(files).map((f) => ({
        id: crypto.randomUUID(),
        name: f.name,
        size: f.size,
        file_type: f.type || "application/octet-stream",
        sha256: null,
      }));

      const prepareResp = await fetch(`${base}/api/prepare-send`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          sender: myDevice,
          files: fileMetas,
          pin: null,
        }),
      });

      if (!prepareResp.ok) {
        throw new Error(`Prepare failed: ${prepareResp.status}`);
      }

      const prepareData = await prepareResp.json();

      // Check if the receiver needs manual confirmation (accepted: false, session_id present)
      if (!prepareData.accepted && prepareData.session_id) {
        // Waiting for receiver to accept - update status
        setTransfers((prev) =>
          prev.map((t) =>
            t.id === transferId
              ? { ...t, status: "waiting_accept", sessionId: prepareData.session_id }
              : t
          )
        );

        // Wait for WS notification (transfer_response with accepted: true)
        // For simplicity, we'll poll the session status
        // The WS handler will update the status when transfer_response comes
        return;
      }

      if (!prepareData.accepted && !prepareData.session_id) {
        setTransfers((prev) =>
          prev.map((t) =>
            t.id === transferId ? { ...t, status: "rejected" } : t
          )
        );
        return;
      }

      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId
            ? { ...t, status: "transferring", sessionId: prepareData.session_id }
            : t
        )
      );

      // 2. Upload each file via multipart
      let bytesDone = 0;
      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        const formData = new FormData();
        formData.append("file", file);

        const resp = await fetch(`${base}/api/send`, {
          method: "POST",
          body: formData,
        });

        if (resp.ok) {
          bytesDone += file.size;
          setTransfers((prev) =>
            prev.map((t) =>
              t.id === transferId
                ? { ...t, bytesTransferred: bytesDone }
                : t
            )
          );
        } else {
          throw new Error(`Upload failed for ${file.name}`);
        }
      }

      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId ? { ...t, status: "completed" } : t
        )
      );
    } catch (err) {
      console.error("Send failed:", err);
      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId ? { ...t, status: "error", error: err.message } : t
        )
      );
    }
  }, [myDevice]);

  // ── scan (triggered by user) ────────────────────────────
  const scan = useCallback(async () => {
    try {
      const resp = await fetch(`${API_BASE}/api/devices`);
      const list = await resp.json();
      if (Array.isArray(list)) {
        setDevices(list);
      }
    } catch (e) {
      console.error("Scan failed:", e);
    }
  }, []);

  // ── update settings ────────────────────────────────────
  const updateSettings = useCallback(async (newSettings) => {
    try {
      const resp = await fetch(`${API_BASE}/api/settings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(newSettings),
      });
      const updated = await resp.json();
      setSettings(updated);
    } catch (e) {
      console.error("Update settings failed:", e);
    }
  }, []);

  return {
    devices,
    transfers,
    connected,
    myDevice,
    settings,
    sendFiles,
    acceptTransfer,
    rejectTransfer,
    scan,
    updateSettings,
  };
}
