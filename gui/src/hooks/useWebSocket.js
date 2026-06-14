import { useState, useEffect, useRef, useCallback } from "react";

const WS_URL = "ws://localhost:53318/api/ws";
const API_BASE = "http://localhost:53318";
const CHUNK_SIZE = 4 * 1024 * 1024; // 4MB per chunk

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
    wsRef.current = ws;
    console.log("[WS] Connecting to", WS_URL, "readyState:", ws.readyState);

    ws.onopen = () => {
      console.log("[WS] Connected successfully");
      setConnected(true);
    };

    ws.onclose = () => {
      console.log("[WS] Connection closed");
      setConnected(false);
      // Only clear ref if this is still the current ws
      if (wsRef.current === ws) {
        wsRef.current = null;
      }
      reconnectTimer.current = setTimeout(() => connect(), 2000);
    };

    ws.onerror = (e) => {
      console.warn("[WS] Connection error - server may not be ready yet, will retry...", e);
      // Don't call ws.close() here - onclose will fire automatically after onerror
      // and we don't want to trigger a double close / race condition
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        handleMessage(msg);
      } catch (e) {
        console.error("WS parse error:", e);
      }
    };
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
      case "update": {
        // Device info updated (e.g. alias change) - update the device list
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
            direction: "incoming",
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
        const { session_id, accepted } = msg;

        // This is for the LOCAL receiver's UI update (incoming transfers only)
        if (accepted) {
          setTransfers((prev) =>
            prev.map((t) =>
              t.sessionId === session_id && t.direction === "incoming"
                ? { ...t, status: t.status === "pending" ? "receiving" : t.status }
                : t
            )
          );
        } else {
          setTransfers((prev) =>
            prev.map((t) =>
              t.sessionId === session_id && t.direction === "incoming"
                ? { ...t, status: "rejected" }
                : t
            )
          );
        }
        break;
      }
      case "progress": {
        // Real-time progress update from server (only for incoming transfers)
        const p = msg.progress;
        setTransfers((prev) =>
          prev.map((t) =>
            t.sessionId === p.session_id && t.direction === "incoming"
              ? {
                  ...t,
                  bytesTransferred: p.bytes_sent,
                  totalBytes: p.total_bytes || t.totalBytes,
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
            t.sessionId === msg.session_id && t.direction === "incoming"
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

  // ── actual upload function (chunked for large files) ─────
  const doUpload = useCallback(async (sessionId, device, files, transferId) => {
    const base = `http://${device.ip}:${device.port}`;

    setTransfers((prev) =>
      prev.map((t) =>
        t.id === transferId
          ? { ...t, status: "transferring", sessionId }
          : t
      )
    );

    try {
      let totalBytesDone = 0;
      let lastSpeedTime = Date.now();
      let lastSpeedBytes = 0;

      for (let i = 0; i < files.length; i++) {
        const file = files[i];

        // For small files (< CHUNK_SIZE * 2), use the legacy multipart upload
        if (file.size < CHUNK_SIZE * 2) {
          const formData = new FormData();
          formData.append("session_id", sessionId);
          formData.append("file", file);

          const resp = await fetch(`${base}/api/send`, {
            method: "POST",
            body: formData,
          });

          if (resp.ok) {
            totalBytesDone += file.size;
            setTransfers((prev) =>
              prev.map((t) =>
                t.id === transferId
                  ? { ...t, bytesTransferred: totalBytesDone }
                  : t
              )
            );
          } else {
            throw new Error(`Upload failed for ${file.name}`);
          }
          continue;
        }

        // Chunked upload for large files
        const totalChunks = Math.ceil(file.size / CHUNK_SIZE);

        for (let chunkIdx = 0; chunkIdx < totalChunks; chunkIdx++) {
          const start = chunkIdx * CHUNK_SIZE;
          const end = Math.min(start + CHUNK_SIZE, file.size);
          const chunkBlob = file.slice(start, end);
          const chunkData = await chunkBlob.arrayBuffer();

          const isFileDone = chunkIdx === totalChunks - 1;
          const isSessionDone = isFileDone && i === files.length - 1;

          const resp = await fetch(
            `${base}/api/upload-chunk?session_id=${encodeURIComponent(sessionId)}&file_name=${encodeURIComponent(file.name)}&chunk_index=${chunkIdx}&total_chunks=${totalChunks}&is_file_done=${isFileDone}&is_session_done=${isSessionDone}`,
            {
              method: "POST",
              headers: {
                "Content-Type": "application/octet-stream",
              },
              body: chunkData,
            }
          );

          if (!resp.ok) {
            throw new Error(`Chunk upload failed for ${file.name} chunk ${chunkIdx}`);
          }

          totalBytesDone += chunkData.byteLength;

          // Calculate speed
          const now = Date.now();
          const elapsed = (now - lastSpeedTime) / 1000;
          let speedBps = 0;
          if (elapsed >= 0.5) {
            speedBps = Math.round((totalBytesDone - lastSpeedBytes) / elapsed);
            lastSpeedTime = now;
            lastSpeedBytes = totalBytesDone;
          }

          setTransfers((prev) =>
            prev.map((t) =>
              t.id === transferId
                ? { ...t, bytesTransferred: totalBytesDone, speed: speedBps || t.speed }
                : t
            )
          );
        }
      }

      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId ? { ...t, status: "completed", bytesTransferred: t.totalBytes } : t
        )
      );
    } catch (err) {
      console.error("Upload error:", err);
      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId ? { ...t, status: "error", error: err.message } : t
        )
      );
    }
  }, []);

  // ── initial connection ─────────────────────────────────
  useEffect(() => {
    let cancelled = false;

    fetch(`${API_BASE}/api/devices`)
      .then((r) => r.json())
      .then((list) => {
        if (!cancelled && Array.isArray(list) && list.length > 0) {
          setDevices(list);
        }
      })
      .catch(() => {});

    fetch(`${API_BASE}/api/settings`)
      .then((r) => r.json())
      .then((s) => {
        if (!cancelled) setSettings(s);
      })
      .catch(() => {});

    // Small delay to ensure server is ready (Tauri spawns it async)
    const initTimer = setTimeout(() => {
      if (!cancelled) connect();
    }, 500);

    return () => {
      cancelled = true;
      clearTimeout(initTimer);
      clearTimeout(reconnectTimer.current);
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
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
        direction: "outgoing",
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
        speed: 0,
      },
    ]);

    try {
      const base = `http://${device.ip}:${device.port}`;

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
      const sessionId = prepareData.session_id;

      if (!sessionId) {
        throw new Error("No session_id returned");
      }

      // Update transfer with session ID
      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId ? { ...t, sessionId } : t
        )
      );

      if (prepareData.accepted) {
        // Auto-accepted - upload immediately
        await doUpload(sessionId, device, Array.from(files), transferId);
        return;
      }

      // Receiver needs to confirm - poll the remote device for session status
      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId ? { ...t, status: "waiting_accept" } : t
        )
      );

      // Poll the remote device's session-status endpoint
      const maxPolls = 120; // 2 minutes max (1s interval)
      let accepted = false;
      for (let i = 0; i < maxPolls; i++) {
        await new Promise((r) => setTimeout(r, 1000));

        try {
          const statusResp = await fetch(`${base}/api/session-status/${sessionId}`);
          if (statusResp.ok) {
            const statusData = await statusResp.json();
            if (statusData.status === "accepted") {
              accepted = true;
              break;
            } else if (statusData.status === "cancelled") {
              setTransfers((prev) =>
                prev.map((t) =>
                  t.id === transferId ? { ...t, status: "rejected" } : t
                )
              );
              return;
            }
          }
        } catch (e) {
          // Network error, continue polling
        }
      }

      if (!accepted) {
        setTransfers((prev) =>
          prev.map((t) =>
            t.id === transferId ? { ...t, status: "error", error: "Timed out waiting for acceptance" } : t
          )
        );
        return;
      }

      // Accepted - start upload
      await doUpload(sessionId, device, Array.from(files), transferId);
    } catch (err) {
      console.error("Send failed:", err);
      setTransfers((prev) =>
        prev.map((t) =>
          t.id === transferId ? { ...t, status: "error", error: err.message } : t
        )
      );
    }
  }, [myDevice, doUpload]);

  // ── scan (triggered by user) ────────────────────────────
  const scan = useCallback(async () => {
    try {
      // Trigger multicast announcement to discover devices on the network
      const scanResp = await fetch(`${API_BASE}/api/scan`, { method: "POST" });
      if (scanResp.ok) {
        const data = await scanResp.json();
        if (Array.isArray(data.devices)) {
          setDevices((prev) => {
            const map = new Map(prev.map((d) => [d.id, d]));
            for (const d of data.devices) map.set(d.id, d);
            return Array.from(map.values());
          });
        }
      }

      // Also refresh from the server's known peers list
      const resp = await fetch(`${API_BASE}/api/devices`);
      const list = await resp.json();
      if (Array.isArray(list)) {
        setDevices((prev) => {
          const map = new Map(prev.map((d) => [d.id, d]));
          for (const d of list) map.set(d.id, d);
          return Array.from(map.values());
        });
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
