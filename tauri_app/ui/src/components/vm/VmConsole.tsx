import { useCallback, useEffect, useRef, useState } from "react";
import {
  Camera,
  ChevronDown,
  ClipboardPaste,
  CornerDownLeft,
  Eye,
  EyeOff,
  Keyboard,
  Maximize2,
  Minimize2,
  Monitor,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Scan,
  Scaling,
  Terminal,
  X,
} from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import RFB from "@novnc/novnc";
import { VmItem, VmNotice } from "./types";

interface VmConsoleProps {
  vm: VmItem;
  logs: string;
  onClearLogs: () => void;
  onNotice?: (notice: VmNotice) => void;
  showConfigPanel?: boolean;
  onToggleConfigPanel?: () => void;
}

const XK_SHIFT_L = 0xffe1;
const XK_CONTROL_L = 0xffe3;
const XK_ALT_L = 0xffe9;
const XK_SUPER_L = 0xffeb;

interface KeySpec {
  keysym: number;
  code: string;
}

const KEY_PRESETS: Array<{ label: string; hint?: string; keys: KeySpec[] }> = [
  { label: "Esc", keys: [{ keysym: 0xff1b, code: "Escape" }] },
  { label: "Tab", keys: [{ keysym: 0xff09, code: "Tab" }] },
  { label: "Super (Win)", keys: [{ keysym: XK_SUPER_L, code: "MetaLeft" }] },
  {
    label: "Ctrl + C",
    hint: "SIGINT",
    keys: [
      { keysym: XK_CONTROL_L, code: "ControlLeft" },
      { keysym: 0x0063, code: "KeyC" },
    ],
  },
  {
    label: "Ctrl + D",
    hint: "EOF",
    keys: [
      { keysym: XK_CONTROL_L, code: "ControlLeft" },
      { keysym: 0x0064, code: "KeyD" },
    ],
  },
  {
    label: "Ctrl + Z",
    hint: "Suspend",
    keys: [
      { keysym: XK_CONTROL_L, code: "ControlLeft" },
      { keysym: 0x007a, code: "KeyZ" },
    ],
  },
  {
    label: "Alt + Tab",
    keys: [
      { keysym: XK_ALT_L, code: "AltLeft" },
      { keysym: 0xff09, code: "Tab" },
    ],
  },
  {
    label: "Alt + F4",
    keys: [
      { keysym: XK_ALT_L, code: "AltLeft" },
      { keysym: 0xffc1, code: "F4" },
    ],
  },
  {
    label: "Ctrl + Alt + F1",
    hint: "TTY1",
    keys: [
      { keysym: XK_CONTROL_L, code: "ControlLeft" },
      { keysym: XK_ALT_L, code: "AltLeft" },
      { keysym: 0xffbe, code: "F1" },
    ],
  },
  {
    label: "Ctrl + Alt + F2",
    hint: "TTY2",
    keys: [
      { keysym: XK_CONTROL_L, code: "ControlLeft" },
      { keysym: XK_ALT_L, code: "AltLeft" },
      { keysym: 0xffbf, code: "F2" },
    ],
  },
  {
    label: "Ctrl + Alt + F3",
    hint: "TTY3",
    keys: [
      { keysym: XK_CONTROL_L, code: "ControlLeft" },
      { keysym: XK_ALT_L, code: "AltLeft" },
      { keysym: 0xffc0, code: "F3" },
    ],
  },
];

const SHIFTED_CHARS = new Set('~!@#$%^&*()_+{}|:"<>?ABCDEFGHIJKLMNOPQRSTUVWXYZ');

export function VmConsole({
  vm,
  logs,
  onClearLogs,
  onNotice,
  showConfigPanel,
  onToggleConfigPanel,
}: VmConsoleProps) {
  const [consoleTab, setConsoleTab] = useState<"vnc" | "logs">("vnc");
  const [vncStatus, setVncStatus] = useState<"idle" | "connecting" | "connected" | "disconnected" | "error">("idle");
  const [reconnectTick, setReconnectTick] = useState(0);

  // Display & interaction controls
  const [scaleMode, setScaleMode] = useState<"fit" | "fill" | "native">("fit");
  const [autoResizeSession, setAutoResizeSession] = useState(false);
  const [viewOnly, setViewOnly] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [guestRes, setGuestRes] = useState<{ w: number; h: number } | null>(null);
  const [keysMenuOpen, setKeysMenuOpen] = useState(false);
  const [pasteBarOpen, setPasteBarOpen] = useState(false);
  const [pasteText, setPasteText] = useState("");
  const [pastePressEnter, setPastePressEnter] = useState(false);

  const vncStageRef = useRef<HTMLDivElement>(null);
  const vncContainerRef = useRef<HTMLDivElement>(null);
  const rfbRef = useRef<RFB | null>(null);
  const logEndRef = useRef<HTMLDivElement>(null);
  const keysMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (consoleTab === "logs" && logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, consoleTab]);

  // Close special-keys dropdown on outside click
  useEffect(() => {
    if (!keysMenuOpen) return;
    const onDocDown = (e: MouseEvent) => {
      if (keysMenuRef.current && !keysMenuRef.current.contains(e.target as Node)) {
        setKeysMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocDown);
    return () => document.removeEventListener("mousedown", onDocDown);
  }, [keysMenuOpen]);

  // Ensure native Tauri fullscreen resets on unmount if left open
  useEffect(() => {
    return () => {
      getCurrentWindow().setFullscreen(false).catch(() => {});
    };
  }, []);

  // Sync RFB options dynamically without tearing down the VNC session
  useEffect(() => {
    const rfb = rfbRef.current;
    if (!rfb) return;
    rfb.scaleViewport = scaleMode !== "native";
    rfb.clipViewport = false;
    rfb.resizeSession = autoResizeSession;
    rfb.viewOnly = viewOnly;
    const t = setTimeout(() => window.dispatchEvent(new Event("resize")), 40);
    return () => clearTimeout(t);
  }, [scaleMode, autoResizeSession, viewOnly]);

  // Keep noVNC scaled to container size & track guest framebuffer dimensions
  useEffect(() => {
    const stage = vncStageRef.current;
    if (!stage || consoleTab !== "vnc") return;

    let rafId: number | null = null;
    const updateMetrics = () => {
      const canvas = vncContainerRef.current?.querySelector("canvas");
      if (canvas && canvas.width > 0 && canvas.height > 0) {
        setGuestRes((prev) =>
          prev && prev.w === canvas.width && prev.h === canvas.height
            ? prev
            : { w: canvas.width, h: canvas.height }
        );
      }
    };

    const obs = new ResizeObserver(() => {
      if (rafId !== null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        window.dispatchEvent(new Event("resize"));
        updateMetrics();
      });
    });
    obs.observe(stage);

    const pollTimer = setInterval(updateMetrics, 1200);
    return () => {
      obs.disconnect();
      clearInterval(pollTimer);
      if (rafId !== null) cancelAnimationFrame(rafId);
    };
  }, [consoleTab, vncStatus]);

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
      setGuestRes(null);
    };

    if (vm.status === "running" && vm.ws_port && consoleTab === "vnc") {
      setVncStatus("connecting");
      const targetUrl = `ws://127.0.0.1:${vm.ws_port}`;

      connectTimeout = setTimeout(() => {
        if (!rfbRef.current && vncContainerRef.current) {
          try {
            const rfb = new RFB(vncContainerRef.current, targetUrl);
            rfb.scaleViewport = scaleMode !== "native";
            rfb.clipViewport = false;
            rfb.resizeSession = autoResizeSession;
            rfb.viewOnly = viewOnly;
            rfb.addEventListener("connect", () => {
              setVncStatus("connected");
              setTimeout(() => window.dispatchEvent(new Event("resize")), 80);
            });
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

  const toggleFullscreen = useCallback(async () => {
    const next = !isFullscreen;
    setIsFullscreen(next);
    try {
      await getCurrentWindow().setFullscreen(next);
    } catch {
      // In-window fixed fullscreen overlay still works if native API is unavailable
    }
    setTimeout(() => {
      window.dispatchEvent(new Event("resize"));
      rfbRef.current?.focus();
    }, 80);
  }, [isFullscreen]);

  const sendKeyCombo = (keys: KeySpec[]) => {
    const rfb = rfbRef.current;
    if (!rfb) return;
    if (viewOnly) {
      onNotice?.({ type: "info", message: "View-only mode is enabled." });
      return;
    }
    for (const k of keys) {
      rfb.sendKey(k.keysym, k.code, true);
    }
    for (let i = keys.length - 1; i >= 0; i--) {
      rfb.sendKey(keys[i].keysym, keys[i].code, false);
    }
    rfb.focus();
  };

  const sendTextToVm = (raw: string, withEnter: boolean) => {
    const rfb = rfbRef.current;
    if (!rfb || !raw) return;
    if (viewOnly) {
      onNotice?.({ type: "info", message: "View-only mode is enabled." });
      return;
    }
    const normalized = raw.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
    for (const ch of normalized) {
      if (ch === "\n") {
        rfb.sendKey(0xff0d, "Enter", true);
        rfb.sendKey(0xff0d, "Enter", false);
        continue;
      }
      if (ch === "\t") {
        rfb.sendKey(0xff09, "Tab", true);
        rfb.sendKey(0xff09, "Tab", false);
        continue;
      }
      const code = ch.charCodeAt(0);
      if (code < 0x20 || code > 0x7e) continue;
      const needShift = SHIFTED_CHARS.has(ch);
      if (needShift) rfb.sendKey(XK_SHIFT_L, "ShiftLeft", true);
      rfb.sendKey(code, `Key_${code}`, true);
      rfb.sendKey(code, `Key_${code}`, false);
      if (needShift) rfb.sendKey(XK_SHIFT_L, "ShiftLeft", false);
    }
    if (withEnter) {
      rfb.sendKey(0xff0d, "Enter", true);
      rfb.sendKey(0xff0d, "Enter", false);
    }
    rfb.focus();
  };

  const handlePasteFromClipboard = async () => {
    try {
      const clip = await navigator.clipboard.readText();
      if (clip) {
        setPasteText(clip);
        setPasteBarOpen(true);
        return;
      }
    } catch {
      // Fallback to manual paste input bar
    }
    setPasteBarOpen((v) => !v);
  };

  const handleScreenshot = () => {
    const canvas = vncContainerRef.current?.querySelector("canvas");
    if (!canvas) {
      onNotice?.({ type: "error", message: "No VNC frame available to capture." });
      return;
    }
    try {
      const dataUrl = canvas.toDataURL("image/png");
      const link = document.createElement("a");
      const safeName = vm.name.replace(/[^a-zA-Z0-9_-]/g, "_");
      link.download = `${safeName}_${Date.now()}.png`;
      link.href = dataUrl;
      link.click();
      onNotice?.({ type: "success", message: `Captured VM screenshot (${canvas.width}×${canvas.height}).` });
    } catch (e) {
      onNotice?.({ type: "error", message: `Screenshot failed: ${e}` });
    }
  };

  const isRunning = vm.status === "running";

  return (
    <div className={`vm-console ${isFullscreen ? "fullscreen" : ""}`}>
      <div className="vm-console-tabs">
        {!isFullscreen && onToggleConfigPanel && (
          <button
            type="button"
            onClick={onToggleConfigPanel}
            className={`vm-btn vm-btn-icon ${showConfigPanel ? "vm-btn-active" : ""}`}
            style={{ marginLeft: "6px" }}
            title={showConfigPanel ? "Hide config panel" : "Show config panel"}
          >
            {showConfigPanel ? <PanelLeftClose size={12} /> : <PanelLeftOpen size={12} />}
            <span>Config</span>
          </button>
        )}
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

        {consoleTab === "vnc" && (
          <div className="vm-console-controls">
            {isRunning && (
              <>
                <span
                  style={{
                    fontSize: "10px",
                    color:
                      vncStatus === "connected"
                        ? "var(--color-success, #22c55e)"
                        : "var(--text-muted)",
                    fontFamily: "var(--font-mono, monospace)",
                    marginRight: "4px",
                    whiteSpace: "nowrap",
                  }}
                  title={`ws://127.0.0.1:${vm.ws_port ?? "?"} (${vncStatus})`}
                >
                  {guestRes ? `${guestRes.w}×${guestRes.h}` : vncStatus}
                </span>

                {/* Viewport Scale: Fit (aspect) | Fill (full frame) | 1:1 (native) */}
                <div className="vm-scale-group" title="Display scaling mode">
                  <button
                    type="button"
                    onClick={() => setScaleMode("fit")}
                    className={`vm-scale-btn ${scaleMode === "fit" ? "active" : ""}`}
                    title="Fit to viewport (preserve aspect ratio)"
                  >
                    <Scan size={10} />
                    <span>Fit</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setScaleMode("fill")}
                    className={`vm-scale-btn ${scaleMode === "fill" ? "active" : ""}`}
                    title="Fill viewport (stretch to 100% of frame)"
                  >
                    <span>Fill</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setScaleMode("native")}
                    className={`vm-scale-btn ${scaleMode === "native" ? "active" : ""}`}
                    title="Native 1:1 pixel size"
                  >
                    <span>1:1</span>
                  </button>
                </div>

                {/* Guest Auto-Resize */}
                <button
                  type="button"
                  onClick={() => setAutoResizeSession((v) => !v)}
                  className={`vm-btn vm-btn-icon ${autoResizeSession ? "vm-btn-active" : ""}`}
                  title="Auto-resize guest resolution to match container"
                >
                  <Scaling size={11} />
                  <span>Auto</span>
                </button>

                {/* View-Only Lock */}
                <button
                  type="button"
                  onClick={() => setViewOnly((v) => !v)}
                  className={`vm-btn vm-btn-icon ${viewOnly ? "vm-btn-active" : ""}`}
                  title={viewOnly ? "View-only mode enabled (click to unlock input)" : "Lock input (view-only)"}
                >
                  {viewOnly ? <Eye size={11} /> : <EyeOff size={11} />}
                </button>

                <span className="vm-toolbar-divider" />

                {/* Ctrl+Alt+Del */}
                <button
                  type="button"
                  onClick={() => {
                    if (viewOnly) return;
                    rfbRef.current?.sendCtrlAltDel();
                    rfbRef.current?.focus();
                  }}
                  className="vm-btn vm-btn-icon"
                  title="Send Ctrl + Alt + Del"
                >
                  <span>CAD</span>
                </button>

                {/* Special Keys Menu */}
                <div ref={keysMenuRef} className="vm-keys-dropdown">
                  <button
                    type="button"
                    onClick={() => setKeysMenuOpen((o) => !o)}
                    className={`vm-btn vm-btn-icon ${keysMenuOpen ? "vm-btn-active" : ""}`}
                    title="Send special key combinations"
                  >
                    <Keyboard size={11} />
                    <span>Keys</span>
                    <ChevronDown size={10} />
                  </button>
                  {keysMenuOpen && (
                    <div className="vm-keys-menu">
                      {KEY_PRESETS.map((preset) => (
                        <button
                          key={preset.label}
                          type="button"
                          className="vm-keys-menu-item"
                          onClick={() => {
                            sendKeyCombo(preset.keys);
                            setKeysMenuOpen(false);
                          }}
                        >
                          <span>{preset.label}</span>
                          {preset.hint && (
                            <span style={{ fontSize: "9px", color: "var(--text-muted)" }}>
                              {preset.hint}
                            </span>
                          )}
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                {/* Paste / Type Text into VM */}
                <button
                  type="button"
                  onClick={handlePasteFromClipboard}
                  className={`vm-btn vm-btn-icon ${pasteBarOpen ? "vm-btn-active" : ""}`}
                  title="Paste or type ASCII text into VM"
                >
                  <ClipboardPaste size={11} />
                </button>

                {/* Screenshot */}
                <button
                  type="button"
                  onClick={handleScreenshot}
                  className="vm-btn vm-btn-icon"
                  title="Capture VM screenshot (PNG)"
                >
                  <Camera size={11} />
                </button>

                {/* Reconnect */}
                <button
                  type="button"
                  onClick={() => setReconnectTick((t) => t + 1)}
                  className="vm-btn vm-btn-icon"
                  title="Reconnect VNC"
                >
                  <RefreshCw size={11} />
                </button>
              </>
            )}

            {/* Fullscreen */}
            <button
              type="button"
              onClick={toggleFullscreen}
              className={`vm-btn vm-btn-icon ${isFullscreen ? "vm-btn-active" : ""}`}
              title={isFullscreen ? "Exit fullscreen" : "Fullscreen VNC display"}
            >
              {isFullscreen ? <Minimize2 size={11} /> : <Maximize2 size={11} />}
              {isFullscreen && <span>Exit</span>}
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

      {consoleTab === "vnc" && pasteBarOpen && isRunning && (
        <div className="vm-paste-bar">
          <input
            type="text"
            value={pasteText}
            onChange={(e) => setPasteText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && pasteText) {
                sendTextToVm(pasteText, pastePressEnter);
                setPasteText("");
              }
            }}
            placeholder="Type or paste ASCII text to send to VM..."
            className="vm-input"
            style={{ flex: 1, fontSize: "11px", padding: "4px 8px" }}
          />
          <label
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "4px",
              fontSize: "11px",
              color: "var(--text-muted)",
              cursor: "pointer",
              whiteSpace: "nowrap",
            }}
          >
            <input
              type="checkbox"
              checked={pastePressEnter}
              onChange={(e) => setPastePressEnter(e.target.checked)}
            />
            +Enter
          </label>
          <button
            type="button"
            disabled={!pasteText}
            onClick={() => {
              sendTextToVm(pasteText, pastePressEnter);
              setPasteText("");
            }}
            className="vm-btn vm-btn-primary vm-btn-icon"
          >
            <CornerDownLeft size={11} /> Send
          </button>
          <button
            type="button"
            onClick={() => setPasteBarOpen(false)}
            className="vm-btn vm-btn-icon"
            title="Close"
          >
            <X size={11} />
          </button>
        </div>
      )}

      {consoleTab === "vnc" ? (
        <div ref={vncStageRef} className={`vm-vnc-stage ${scaleMode}`}>
          {isRunning ? (
            <div ref={vncContainerRef} className="vm-vnc-container" />
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
