import { useEffect, useRef, useState } from "react";
import { Monitor, RefreshCw, Terminal } from "lucide-react";
import RFB from "@novnc/novnc";
import { VmItem } from "./types";

interface VmConsoleProps {
  vm: VmItem;
  logs: string;
  onClearLogs: () => void;
}

export function VmConsole({ vm, logs, onClearLogs }: VmConsoleProps) {
  const [consoleTab, setConsoleTab] = useState<"vnc" | "logs">("vnc");
  const [vncStatus, setVncStatus] = useState<"idle" | "connecting" | "connected" | "disconnected" | "error">("idle");
  const [reconnectTick, setReconnectTick] = useState(0);
  const vncContainerRef = useRef<HTMLDivElement>(null);
  const rfbRef = useRef<RFB | null>(null);
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (consoleTab === "logs" && logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, consoleTab]);

  useEffect(() => {
    let connectTimeout: ReturnType<typeof setTimeout> | undefined;

    const cleanupRfb = () => {
      if (rfbRef.current) {
        try {
          rfbRef.current.disconnect();
        } catch {
          // ignore disconnect errors
        }
        rfbRef.current = null;
      }
    };

    if (vm.status === "running" && vm.ws_port && consoleTab === "vnc") {
      setVncStatus("connecting");
      const targetUrl = `ws://127.0.0.1:${vm.ws_port}`;

      connectTimeout = setTimeout(() => {
        if (!rfbRef.current && vncContainerRef.current) {
          try {
            const rfb = new RFB(vncContainerRef.current, targetUrl);
            rfb.scaleViewport = true;
            rfb.resizeSession = true;
            rfb.addEventListener("connect", () => setVncStatus("connected"));
            rfb.addEventListener("disconnect", () => setVncStatus("disconnected"));
            rfb.addEventListener("securityfailure", () => setVncStatus("error"));
            rfbRef.current = rfb;
          } catch (e) {
            console.error("VNC connection failed:", e);
            setVncStatus("error");
          }
        }
      }, 600);
    } else {
      cleanupRfb();
      setVncStatus("idle");
    }

    return () => {
      if (connectTimeout) clearTimeout(connectTimeout);
      cleanupRfb();
    };
  }, [vm.id, vm.status, vm.ws_port, consoleTab, reconnectTick]);

  return (
    <div className="vm-console">
      <div className="vm-console-tabs">
        <button
          type="button"
          onClick={() => setConsoleTab("vnc")}
          className={`vm-console-tab ${consoleTab === "vnc" ? "active" : ""}`}
        >
          <Monitor size={12} /> VM Display (VNC)
        </button>
        <button
          type="button"
          onClick={() => setConsoleTab("logs")}
          className={`vm-console-tab ${consoleTab === "logs" ? "active" : ""}`}
        >
          <Terminal size={12} /> Serial &amp; QEMU Logs
        </button>

        {consoleTab === "vnc" && vm.status === "running" && (
          <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: "8px", paddingRight: "10px" }}>
            <span style={{ fontSize: "10px", color: "var(--text-muted)", fontFamily: "var(--font-mono, monospace)" }}>
              ws://127.0.0.1:{vm.ws_port ?? "?"} ({vncStatus})
            </span>
            <button
              type="button"
              onClick={() => setReconnectTick((t) => t + 1)}
              className="vm-btn"
              style={{ padding: "2px 7px", fontSize: "10px" }}
              title="Reconnect VNC"
            >
              <RefreshCw size={10} /> Reconnect
            </button>
          </div>
        )}

        {consoleTab === "logs" && (
          <button
            type="button"
            onClick={onClearLogs}
            style={{
              marginLeft: "auto",
              background: "none",
              border: "none",
              color: "var(--text-muted)",
              fontSize: "11px",
              cursor: "pointer",
              padding: "0 12px",
            }}
          >
            Clear
          </button>
        )}
      </div>

      {consoleTab === "vnc" ? (
        <div
          style={{
            flex: 1,
            position: "relative",
            overflow: "hidden",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "#111",
          }}
        >
          {vm.status === "running" ? (
            <div ref={vncContainerRef} style={{ width: "100%", height: "100%" }} />
          ) : (
            <div style={{ color: "var(--text-muted)", fontSize: "12px" }}>
              VM is not running. Click &quot;Start VM&quot; to boot.
            </div>
          )}
        </div>
      ) : (
        <div
          style={{
            flex: 1,
            padding: "10px 12px",
            overflowY: "auto",
            fontFamily: "var(--font-mono, monospace)",
            fontSize: "12px",
            color: "#10b981",
            backgroundColor: "var(--terminal-bg, #050507)",
            whiteSpace: "pre-wrap",
          }}
        >
          {logs}
          <div ref={logEndRef} />
        </div>
      )}
    </div>
  );
}
