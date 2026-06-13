import { useState, useEffect, useRef, useCallback } from "react";

const WS_URL = "ws://localhost:53318/api/ws";
const API_BASE = "http://localhost:53318";

/**
 * Custom hook: manages WebSocket connection to QuickShare backend.
 * Returns devices, transfers, and send function.
 */
export function useQuickShare() {
  const [devices, setDevices] = useState([]);
  const [transfers, setTransfers] = useState([]);
  const [connected, setConnected] = useState(false);
  const [myDevice, setMyDevice] = useState(null);
  const wsRef = useRef(null);
  const reconnectTimer = useRef(null);

  // ── connect ────────────────────────────────────────────
  const connect = useCallback(() => {
    if (wsRef.current && wsRef.current.readyState <= 1) return; // already connecting/open

    const ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      setConnected(true);
    };

    ws.onclose = () => {
      setConnected(false);
      wsRef.current = null;
      // auto-reconnect after 2s
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
        // Initialize devices from the peers list the server knows about
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
            id: crypto.randomUUID(),
            sessionId: null,
            from: msg.from,
            files: msg.files,
            status: "pending", // waiting for user to accept
            bytesTransferred: 0,
            totalBytes: msg.files.reduce((s, f) => s + f.size, 0),
          },
        ]);
        break;
      }
      case "transfer_response": {
        // A receiver accepted/rejected
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

    connect();
    return () => {
      clearTimeout(reconnectTimer.current);
      if (wsRef.current) wsRef.current.close();
    };
  }, [connect]);

  // ── accept incoming transfer ───────────────────────────
  const acceptTransfer = useCallback(async (transfer) => {
    // Auto-accept via prepare-send (the server already auto-accepts)
    setTransfers((prev) =>
      prev.map((t) =>
        t.id === transfer.id ? { ...t, status: "receiving" } : t
      )
    );
  }, []);

  // ── reject incoming transfer ───────────────────────────
  const rejectTransfer = useCallback((transfer) => {
    setTransfers((prev) =>
      prev.map((t) =>
        t.id === transfer.id ? { ...t, status: "rejected" } : t
      )
    );
  }, []);

  // ── send files to a device ─────────────────────────────
  const sendFiles = useCallback(async (device, files) => {
    if (!device || !files || files.length === 0) return;

    const transferId = crypto.randomUUID();
    const totalSize = Array.from(files).reduce((s, f) => s + f.size, 0);

    // Add a "sending" transfer to the list
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
      if (!prepareData.accepted) {
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

  return {
    devices,
    transfers,
    connected,
    myDevice,
    sendFiles,
    acceptTransfer,
    rejectTransfer,
    scan,
  };
}
