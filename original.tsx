import React, { useState, useEffect, useRef } from "react";
import { Play, Square, Minus, Square as SquareIcon, X, Plus, Trash2, Server, RefreshCw } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import WindowResizer from "./WindowResizer";
import brandIcon from "./logo_alouette.png";
import { WindowControls } from "./WindowControls";

const DockerIcon = ({ size = 14, className = "", style = {} }: { size?: number, className?: string, style?: React.CSSProperties }) => (
  <svg 
    xmlns="http://www.w3.org/2000/svg" 
    width={size} 
    height={size} 
    viewBox="0 0 24 24" 
    fill="currentColor"
    className={className}
    style={{ marginRight: '8px', ...style }}
  >
    <path d="M13.983 11.078h2.119a.186.186 0 00.186-.185V9.006a.186.186 0 00-.186-.186h-2.119a.185.185 0 00-.185.185v1.888c0 .102.083.185.185.185m-2.954-5.43h2.118a.186.186 0 00.186-.186V3.574a.186.186 0 00-.186-.185h-2.118a.185.185 0 00-.185.185v1.888c0 .102.082.185.185.185m0 2.716h2.118a.187.187 0 00.186-.186V6.29a.186.186 0 00-.186-.185h-2.118a.185.185 0 00-.185.185v1.887c0 .102.082.185.185.186m-2.93 0h2.12a.186.186 0 00.184-.186V6.29a.185.185 0 00-.185-.185H8.1a.185.185 0 00-.185.185v1.887c0 .102.083.185.185.186m-2.964 0h2.119a.186.186 0 00.185-.186V6.29a.185.185 0 00-.185-.185H5.136a.186.186 0 00-.186.185v1.887c0 .102.084.185.186.186m5.893 2.715h2.118a.186.186 0 00.186-.185V9.006a.186.186 0 00-.186-.186h-2.118a.185.185 0 00-.185.185v1.888c0 .102.082.185.185.185m-2.93 0h2.12a.185.185 0 00.184-.185V9.006a.185.185 0 00-.184-.186h-2.12a.185.185 0 00-.184.185v1.888c0 .102.083.185.185.185m-2.964 0h2.119a.185.185 0 00.185-.185V9.006a.185.185 0 00-.184-.186h-2.12a.186.186 0 00-.186.186v1.887c0 .102.084.185.186.185m-2.92 0h2.12a.185.185 0 00.184-.185V9.006a.185.185 0 00-.184-.186h-2.12a.185.185 0 00-.184.185v1.888c0 .102.082.185.185.185M23.763 9.89c-.065-.051-.672-.51-1.954-.51-.338.001-.676.03-1.01.087-.248-1.7-1.653-2.53-1.716-2.566l-.344-.199-.226.327c-.284.438-.49.922-.612 1.43-.23.97-.09 1.882.403 2.661-.595.332-1.55.413-1.744.42H.751a.751.751 0 00-.75.748 11.376 11.376 0 00.692 4.062c.545 1.428 1.355 2.48 2.41 3.124 1.18.723 3.1 1.137 5.275 1.137.983.003 1.963-.086 2.93-.266a12.248 12.248 0 003.823-1.389c.98-.567 1.86-1.288 2.61-2.136 1.252-1.418 1.998-2.997 2.553-4.4h.221c1.372 0 2.215-.549 2.68-1.009.309-.293.55-.65.707-1.046l.098-.288Z"/>
  </svg>
);

const THEME = {
  bgApp: "#1e1e1e",
  bgPanel: "#252526",
  bgInput: "#3c3c3c",
  bgHover: "#2a2d2e",
  border: "#3c3c3c",
  borderFocus: "#007acc",
  textMain: "#cccccc",
  textMuted: "#858585",
  accent: "#007acc",
  accentHover: "#005a9e",
  success: "#89d185",
  error: "#f48771",
  warning: "#cca700"
};

const inputStyle: React.CSSProperties = {
  padding: "4px 8px",
  backgroundColor: THEME.bgInput,
  border: `1px solid ${THEME.border}`,
  color: THEME.textMain,
  outline: "none",
  fontSize: "12px",
  fontFamily: "monospace",
  width: "100%",
  boxSizing: "border-box"
};

const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: "11px",
  fontWeight: "bold",
  color: THEME.textMuted,
  marginBottom: "4px",
  textTransform: "uppercase",
};

const btnStyle: React.CSSProperties = {
  padding: "4px 12px",
  backgroundColor: THEME.bgInput,
  border: `1px solid ${THEME.border}`,
  color: THEME.textMain,
  cursor: "pointer",
  fontSize: "12px",
  display: "flex",
  alignItems: "center",
  gap: "6px"
};

export default function DockerManager() {
  const appWindow = getCurrentWindow();

  const [activeView, setActiveView] = useState<"create" | "manage">("create");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [containers, setContainers] = useState<any[]>([]);
  const [daemonRunning, setDaemonRunning] = useState(false);
  const [checkingDaemon, setCheckingDaemon] = useState(true);
  const [daemonError, setDaemonError] = useState("");

  // Form states
  const [cName, setCName] = useState("");
  const [cImage, setCImage] = useState("ubuntu:latest");
  const [cCmd, setCCmd] = useState("");
  const [cRam, setCRam] = useState(512);

  const [logs, setLogs] = useState("");
  const logEndRef = useRef<HTMLDivElement>(null);
  const [stats, setStats] = useState({ cpu: 0, ram: 0, ramLimit: 0 });
  const [manageTab, setManageTab] = useState<"logs" | "stats" | "settings">("logs");

  const checkDaemon = async () => {
    setCheckingDaemon(true);
    try {
      await invoke("docker_ensure_started");
      setDaemonRunning(true);
      setDaemonError("");
      await loadContainers();
    } catch (e: any) {
      console.error(e);
      setDaemonRunning(false);
      setDaemonError(String(e));
    } finally {
      setCheckingDaemon(false);
    }
  };

  const loadContainers = async () => {
    try {
      const list: any[] = await invoke("docker_list_containers", { all: true });
      setContainers(list);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    checkDaemon();
  }, []);

  useEffect(() => {
    let unlistenLogs: any;
    let unlistenStats: any;
    if (activeView === "manage" && selectedId) {
      setLogs("");
      invoke("docker_stream_logs", { id: selectedId });
      invoke("docker_stream_stats", { id: selectedId });

      listen("docker_log", (event: any) => {
        if (event.payload.id === selectedId) {
          setLogs(prev => prev + event.payload.message);
          if (logEndRef.current) logEndRef.current.scrollIntoView({ behavior: "smooth" });
        }
      }).then(un => unlistenLogs = un);

      listen("docker_stats", (event: any) => {
        if (event.payload.id === selectedId) {
          setStats({
            cpu: event.payload.stats.cpu_percent,
            ram: event.payload.stats.memory_usage_bytes,
            ramLimit: event.payload.stats.memory_limit_bytes
          });
        }
      }).then(un => unlistenStats = un);
    }

    return () => {
      if (unlistenLogs) unlistenLogs();
      if (unlistenStats) unlistenStats();
    }
  }, [activeView, selectedId]);

  const handleCreate = async () => {
    try {
      const config = {
        name: cName,
        image: cImage,
        cmd: cCmd ? cCmd.split(" ") : null,
        env: null,
        port_bindings: null, // Simplification for now
        binds: null,
        memory_bytes: cRam * 1024 * 1024,
        nano_cpus: null
      };
      await invoke("docker_create_container", { config });
      alert("Container created!");
      setCName("");
      setCCmd("");
      await loadContainers();
    } catch (err: any) {
      alert("Failed to create container: " + err);
    }
  };

  const handleAction = async (action: string, id: string) => {
    try {
      if (action === "start") await invoke("docker_start_container", { id });
      if (action === "stop") await invoke("docker_stop_container", { id });
      if (action === "restart") await invoke("docker_restart_container", { id });
      if (action === "remove") {
        if (!confirm("Are you sure you want to remove this container?")) return;
        await invoke("docker_remove_container", { id, force: true });
        setSelectedId(null);
        setActiveView("create");
      }
      await loadContainers();
    } catch (e: any) {
      alert(`Action ${action} failed: ` + e);
    }
  };

  const currentContainer = containers.find(c => c.id === selectedId);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', backgroundColor: THEME.bgApp, color: THEME.textMain, fontFamily: "sans-serif", overflow: "hidden" }}>
      <WindowResizer />
      
      {/* Titlebar */}
      <div data-tauri-drag-region style={{ height: "30px", display: "flex", alignItems: "center", justifyContent: "space-between", backgroundColor: THEME.bgApp, borderBottom: `1px solid ${THEME.border}`, padding: "0 8px", userSelect: "none" }}>
        <div style={{ display: "flex", alignItems: "center", pointerEvents: "none" }}>
          <img src={brandIcon} alt="Logo" style={{ width: "14px", height: "14px", marginRight: "8px" }} />
          <DockerIcon size={14} />
          <span style={{ fontSize: "12px", fontWeight: "bold" }}>Docker Manager</span>
        </div>
        <WindowControls />
      </div>
      
      {!daemonRunning ? (
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
          <DockerIcon size={48} className="spin" style={{ color: THEME.textMuted, marginBottom: '16px' }} />
          <h3>{checkingDaemon ? "Starting Docker Daemon..." : "Docker Engine is not running"}</h3>
          {daemonError && <div style={{ color: THEME.error, marginTop: '8px', fontSize: '12px', textAlign: 'center', maxWidth: '400px' }}>{daemonError}</div>}
          {!checkingDaemon && <button onClick={checkDaemon} style={{ ...btnStyle, marginTop: '16px', backgroundColor: THEME.accent, color: '#fff', border: 'none' }}><Play size={12}/> Try Again</button>}
        </div>
      ) : (
        <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
          {/* Left Sidebar */}
          <div style={{ width: '240px', backgroundColor: THEME.bgPanel, borderRight: `1px solid ${THEME.border}`, display: 'flex', flexDirection: 'column' }}>
            <div style={{ padding: '8px' }}>
              <button 
                onClick={() => { setActiveView("create"); setSelectedId(null); }}
                style={{ ...btnStyle, width: '100%', justifyContent: 'center', backgroundColor: THEME.bgApp }}
              >
                <Plus size={14} /> Deploy Container
              </button>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '4px 8px', fontSize: '10px', fontWeight: 'bold', color: THEME.textMuted, borderBottom: `1px solid ${THEME.border}` }}>
              CONTAINERS
              <RefreshCw size={10} style={{ cursor: 'pointer' }} onClick={loadContainers} />
            </div>
            <div style={{ flex: 1, overflowY: 'auto' }}>
              {containers.map((c: any) => {
                const isRunning = c.state === "running";
                const name = c.names?.[0]?.replace("/", "") || (c.id ? c.id.substring(0,8) : "Unknown");
                return (
                  <div 
                    key={c.id}
                    onClick={() => { setActiveView("manage"); setSelectedId(c.id); }}
                    style={{ 
                      padding: '6px 8px', 
                      cursor: 'pointer', 
                      display: 'flex', 
                      alignItems: 'center', 
                      justifyContent: 'space-between',
                      backgroundColor: (activeView === "manage" && selectedId === c.id) ? THEME.bgHover : 'transparent',
                      borderLeft: (activeView === "manage" && selectedId === c.id) ? `3px solid ${THEME.accent}` : '3px solid transparent'
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', overflow: 'hidden' }}>
                      <div style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: isRunning ? THEME.success : THEME.textMuted }} />
                      <span style={{ fontSize: '12px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{name}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Main Content Area */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', backgroundColor: THEME.bgApp, overflow: 'hidden' }}>
            {activeView === "manage" && currentContainer ? (
              <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
                {/* Manage Toolbar */}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '8px 12px', backgroundColor: THEME.bgPanel, borderBottom: `1px solid ${THEME.border}` }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span style={{ fontSize: '14px', fontWeight: 'bold', marginRight: '16px' }}>{currentContainer.names?.[0]?.replace("/", "")}</span>
                    {currentContainer.state === 'running' ? (
                      <>
                        <button onClick={() => handleAction("stop", currentContainer.id)} style={{ ...btnStyle, color: THEME.warning, borderColor: THEME.warning }}><Square size={12} fill="currentColor" /> Stop</button>
                        <button onClick={() => handleAction("restart", currentContainer.id)} style={btnStyle}><RefreshCw size={12} /> Restart</button>
                      </>
                    ) : (
                      <button onClick={() => handleAction("start", currentContainer.id)} style={{ ...btnStyle, color: THEME.success, borderColor: THEME.success }}><Play size={12} fill="currentColor" /> Start</button>
                    )}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <button onClick={() => handleAction("remove", currentContainer.id)} style={{ ...btnStyle, color: THEME.error }}><Trash2 size={12} /> Remove</button>
                  </div>
                </div>

                {/* Tabs */}
                <div style={{ display: 'flex', backgroundColor: THEME.bgPanel, borderBottom: `1px solid ${THEME.border}` }}>
                  {["logs", "stats", "settings"].map(tab => (
                    <button 
                      key={tab}
                      onClick={() => setManageTab(tab as any)}
                      style={{ padding: '6px 12px', background: manageTab === tab ? THEME.bgApp : 'transparent', border: 'none', borderBottom: manageTab === tab ? `2px solid ${THEME.accent}` : '2px solid transparent', color: manageTab === tab ? THEME.accent : THEME.textMuted, fontSize: '11px', fontWeight: 'bold', cursor: 'pointer', textTransform: 'uppercase' }}
                    >
                      {tab}
                    </button>
                  ))}
                </div>

                {/* Tab Content */}
                {manageTab === "logs" && (
                  <div style={{ flex: 1, padding: '8px', overflowY: 'auto', backgroundColor: '#000', fontFamily: 'monospace', fontSize: '12px', color: '#ccc', whiteSpace: 'pre-wrap' }}>
                    {logs || "No logs available."}
                    <div ref={logEndRef} />
                  </div>
                )}
                {manageTab === "stats" && (
                  <div style={{ padding: '24px', flex: 1 }}>
                    <h3 style={{ color: THEME.accent }}>Live Metrics</h3>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px', marginTop: '16px' }}>
                      <div style={{ padding: '16px', backgroundColor: THEME.bgPanel, borderRadius: '4px', border: `1px solid ${THEME.border}` }}>
                        <div style={labelStyle}>CPU Usage</div>
                        <div style={{ fontSize: '24px', fontWeight: 'bold' }}>{stats.cpu.toFixed(2)}%</div>
                      </div>
                      <div style={{ padding: '16px', backgroundColor: THEME.bgPanel, borderRadius: '4px', border: `1px solid ${THEME.border}` }}>
                        <div style={labelStyle}>Memory Usage</div>
                        <div style={{ fontSize: '24px', fontWeight: 'bold' }}>{(stats.ram / 1024 / 1024).toFixed(2)} MB / {(stats.ramLimit / 1024 / 1024).toFixed(2)} MB</div>
                      </div>
                    </div>
                  </div>
                )}
                {manageTab === "settings" && (
                  <div style={{ padding: '24px', flex: 1, overflowY: 'auto' }}>
                     <h3 style={{ color: THEME.textMain, marginBottom: '16px' }}>Container Inspect Data</h3>
                     <pre style={{ backgroundColor: THEME.bgPanel, padding: '12px', borderRadius: '4px', border: `1px solid ${THEME.border}`, fontSize: '11px', overflowX: 'auto' }}>
                       {JSON.stringify(currentContainer, null, 2)}
                     </pre>
                  </div>
                )}
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: '32px', alignItems: 'center', overflowY: 'auto' }}>
                <div style={{ width: '100%', maxWidth: '500px' }}>
                  <h2 style={{ fontSize: '20px', marginBottom: '8px', color: THEME.textMain, display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <Server size={20} color={THEME.accent} /> Deploy New Container
                  </h2>
                  <p style={{ fontSize: '12px', color: THEME.textMuted, marginBottom: '24px' }}>Fill out the details below to deploy a new Docker container.</p>
                
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', backgroundColor: THEME.bgPanel, padding: '24px', borderRadius: '4px', border: `1px solid ${THEME.border}` }}>
                    <div>
                      <label style={labelStyle}>Container Name (Optional)</label>
                      <input type="text" value={cName} onChange={e => setCName(e.target.value)} style={inputStyle} placeholder="my-nginx-app" />
                    </div>
                    <div>
                      <label style={labelStyle}>Docker Image</label>
                      <input type="text" value={cImage} onChange={e => setCImage(e.target.value)} style={inputStyle} placeholder="nginx:latest, ubuntu, redis..." />
                    </div>
                    <div>
                      <label style={labelStyle}>Command (Optional)</label>
                      <input type="text" value={cCmd} onChange={e => setCCmd(e.target.value)} style={inputStyle} placeholder="npm start" />
                    </div>
                    <div>
                      <label style={labelStyle}>Memory Limit (MB)</label>
                      <input type="number" value={cRam} onChange={e => setCRam(Number(e.target.value))} style={inputStyle} />
                    </div>
                    <button onClick={handleCreate} style={{ ...btnStyle, backgroundColor: THEME.accent, color: '#fff', fontWeight: 'bold', justifyContent: 'center', marginTop: '8px' }}>
                      <Play size={14} /> Deploy
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}


