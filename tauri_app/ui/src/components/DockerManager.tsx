import React, { useState, useEffect, useRef, useMemo } from "react";
import { Play, Square, Trash2, Server, RefreshCw, Plus } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Terminal } from "xterm";
import { FitAddon } from "@xterm/addon-fit";
import "xterm/css/xterm.css";
import WindowResizer from "./WindowResizer";
import brandIcon from "./logo_alouette.png";
import { WindowControls } from "./WindowControls";

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
  textTransform: "uppercase"
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

export const DockerIcon = ({ size = 14, className = "", style = {} }: { size?: number, className?: string, style?: React.CSSProperties }) => (
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

const useContainerStats = (containerId: string | null) => {
  const [stats, setStats] = useState({ cpu: 0, ram: 0 });

  useEffect(() => {
    if (!containerId) return;
    
    let unlisten: any;
    invoke("docker_stream_stats", { id: containerId }).catch(console.error);

    listen("docker_stats", (event: any) => {
      if (event.payload.id === containerId) {
        setStats({
          cpu: event.payload.stats.cpu_percent || 0,
          ram: (event.payload.stats.memory_usage_bytes || 0) / (1024 * 1024)
        });
      }
    }).then(un => unlisten = un).catch(console.error);

    return () => {
      if (unlisten) unlisten();
    };
  }, [containerId]);

  return stats;
};

const LogsTab = ({ containerId, since, onClear }: { containerId: string, since: number, onClear: () => void }) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const termInstance = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!terminalRef.current) return;
    const term = new Terminal({
      theme: { background: '#1e1e1e', foreground: '#cccccc' },
      fontSize: 12,
      fontFamily: 'Consolas, "Courier New", monospace',
      disableStdin: true,
      convertEol: true,
    });
    termInstance.current = term;
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalRef.current);
    try { fitAddon.fit(); } catch (e) { console.warn(e); }

    let unlisten: any;
    invoke("docker_stream_logs", { id: containerId, since }).catch(console.error);
    listen("docker_log", (event: any) => {
      if (event.payload.id === containerId) {
        let msg = event.payload.message;
        msg = msg.replace(/ (INFO|info) /g, '\x1b[32m$1\x1b[0m');
        msg = msg.replace(/ (WARN|warn|WARNING|warning) /g, '\x1b[33m$1\x1b[0m');
        msg = msg.replace(/ (ERROR|error|ERR|err) /g, '\x1b[31m$1\x1b[0m');
        msg = msg.replace(/ (DEBUG|debug) /g, '\x1b[34m$1\x1b[0m');
        msg = msg.replace(/ \d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?Z? /g, '\x1b[36m$&\x1b[0m');
        term.write(msg);
      }
    }).then(un => unlisten = un).catch(console.error);

    const handleResize = () => { try { fitAddon.fit(); } catch (e) {} };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (unlisten) unlisten();
      term.dispose();
      termInstance.current = null;
    };
  }, [containerId, since]);

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%' }}>
      <button 
        onClick={() => {
          termInstance.current?.clear();
          onClear();
        }} 
        style={{ ...btnStyle, position: 'absolute', top: 8, right: 24, zIndex: 10, opacity: 0.7 }}
        onMouseEnter={(e) => e.currentTarget.style.opacity = '1'}
        onMouseLeave={(e) => e.currentTarget.style.opacity = '0.7'}
      >
        <Trash2 size={12}/> Clear
      </button>
      <div ref={terminalRef} style={{ width: '100%', height: '100%', padding: '8px' }} />
    </div>
  );
};

const TerminalTab = ({ containerId }: { containerId: string }) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const execIdRef = useRef<string | null>(null);
  const termInstance = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!terminalRef.current) return;

    const term = new Terminal({
      theme: { background: '#1e1e1e', foreground: '#cccccc' },
      fontSize: 13,
      fontFamily: 'Consolas, "Courier New", monospace',
      cursorBlink: true,
    });
    termInstance.current = term;
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalRef.current);
    try { fitAddon.fit(); } catch (e) { console.warn(e); }

    let unlisten: any;
    const initTerminal = async () => {
      try {
        const execId: string = await invoke("docker_exec_terminal", { id: containerId });
        execIdRef.current = execId;
        term.onData(async (data) => {
          if (execIdRef.current) {
            await invoke("docker_write_terminal", { execId: execIdRef.current, data }).catch(console.error);
          }
        });
        unlisten = await listen("docker_terminal_out", (event: any) => {
          if (event.payload.exec_id === execIdRef.current) {
            term.write(event.payload.data);
          }
        });
      } catch (e) {
        term.write(`\r\nFailed to attach terminal: ${e}\r\n`);
      }
    };
    initTerminal();

    const handleResize = () => { try { fitAddon.fit(); } catch (e) {} };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (unlisten) unlisten();
      term.dispose();
      termInstance.current = null;
    };
  }, [containerId]);

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%' }}>
      <button 
        onClick={() => termInstance.current?.clear()} 
        style={{ ...btnStyle, position: 'absolute', top: 8, right: 24, zIndex: 10, opacity: 0.7 }}
        onMouseEnter={(e) => e.currentTarget.style.opacity = '1'}
        onMouseLeave={(e) => e.currentTarget.style.opacity = '0.7'}
      >
        <Trash2 size={12}/> Clear
      </button>
      <div ref={terminalRef} style={{ width: '100%', height: '100%', padding: '8px' }} />
    </div>
  );
};

const getC = (c: any) => {
  if (!c) return { id: '', names: [], state: '', image: '' };
  return {
    id: c.Id || c.id,
    names: c.Names || c.names,
    state: c.State || c.state,
    image: c.Image || c.image
  };
};

export default function DockerManager() {
  const [activeView, setActiveView] = useState<"create" | "manage">("manage");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [containers, setContainers] = useState<any[]>([]);
  const [daemonRunning, setDaemonRunning] = useState(false);
  const [checkingDaemon, setCheckingDaemon] = useState(true);
  const [daemonError, setDaemonError] = useState("");

  const [cName, setCName] = useState("");
  const [cImage, setCImage] = useState("");
  const [cCmd, setCCmd] = useState("");
  const [cRam, setCRam] = useState(512);

  const [manageTab, setManageTab] = useState<"logs" | "terminal" | "settings">("logs");
  const [clearedLogsAt, setClearedLogsAt] = useState<Record<string, number>>({});
  const currentStats = useContainerStats(activeView === "manage" ? selectedId : null);

  const checkDaemon = async () => {
    setCheckingDaemon(true);
    try {
      await invoke("docker_ensure_started");
      setDaemonRunning(true);
      setDaemonError("");
      await loadContainers(true);
    } catch (e: any) {
      console.error(e);
      setDaemonRunning(false);
      setDaemonError(String(e));
    } finally {
      setCheckingDaemon(false);
    }
  };

  const loadContainers = async (isInitialLoad = false) => {
    try {
      const list: any[] = await invoke("docker_list_containers", { all: true });
      setContainers(list || []);
      if (isInitialLoad && list && list.length > 0 && !selectedId) {
        setSelectedId(getC(list[0]).id);
        setActiveView("manage");
      }
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    checkDaemon();
    const interval = setInterval(() => {
      if (daemonRunning) loadContainers(false);
    }, 5000);
    return () => clearInterval(interval);
  }, [daemonRunning]);

  const handleCreate = async () => {
    if (!cImage) return alert("Image name is required");
    try {
      const config = {
        name: cName || null,
        image: cImage,
        cmd: cCmd ? cCmd.split(" ") : null,
        env: null,
        port_bindings: null,
        binds: null,
        memory_bytes: cRam * 1024 * 1024,
        nano_cpus: null
      };
      const newId: string = await invoke("docker_create_container", { config });
      setCName("");
      setCCmd("");
      setCImage("");
      await loadContainers(false);
      setSelectedId(newId);
      setActiveView("manage");
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
        // if (!confirm("Are you sure you want to remove this container?")) return;
        await invoke("docker_remove_container", { id, force: true });
        setSelectedId(null);
        if (containers.length <= 1) setActiveView("create");
      }
      await loadContainers();
    } catch (e: any) {
      alert(`Action ${action} failed: ` + e);
    }
  };

  const currentContainer = useMemo(() => containers.find(c => getC(c).id === selectedId), [containers, selectedId]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', backgroundColor: THEME.bgApp, color: THEME.textMain, fontFamily: "'Inter', sans-serif", overflow: "hidden" }}>
      <WindowResizer />
      
      <div data-tauri-drag-region style={{ height: "36px", display: "flex", alignItems: "center", justifyContent: "space-between", backgroundColor: THEME.bgPanel, borderBottom: `1px solid ${THEME.border}`, padding: "0 12px", userSelect: "none" }}>
        <div style={{ display: "flex", alignItems: "center", pointerEvents: "none", gap: "8px" }}>
          <img src={brandIcon} alt="Logo" style={{ width: "16px", height: "16px" }} />
          <DockerIcon size={16} style={{ color: '#ffffff' }} />
          <span style={{ fontSize: "13px", fontWeight: "600", letterSpacing: "0.5px" }}>Docker Desktop</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <WindowControls />
        </div>
      </div>
      
      {!daemonRunning ? (
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
          <DockerIcon size={48} className={checkingDaemon ? "spin" : ""} style={{ color: THEME.accent, margin: '0 0 24px 0' }} />
          <h3 style={{ fontSize: '20px', margin: '0 0 8px 0' }}>{checkingDaemon ? "Starting Docker Engine..." : "Docker Engine is stopped"}</h3>
          {daemonError && <div style={{ color: THEME.error, marginTop: '16px', fontSize: '13px', textAlign: 'center', maxWidth: '500px', backgroundColor: THEME.bgPanel, padding: '12px', borderRadius: '4px', border: `1px solid ${THEME.border}` }}>{daemonError}</div>}
          {!checkingDaemon && <button onClick={checkDaemon} style={{ ...btnStyle, marginTop: '24px', backgroundColor: THEME.accent, color: '#fff', border: 'none', padding: '8px 24px', fontSize: '14px' }}><Play size={16}/> Start Engine</button>}
          <style>{`@keyframes spin { 100% { transform: rotate(360deg); } } .spin { animation: spin 1s linear infinite; }`}</style>
        </div>
      ) : (
        <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
          {/* Sidebar */}
          <div style={{ width: '260px', backgroundColor: THEME.bgPanel, borderRight: `1px solid ${THEME.border}`, display: 'flex', flexDirection: 'column' }}>
            <div style={{ padding: '16px 12px' }}>
              <button 
                onClick={() => { setActiveView("create"); setSelectedId(null); }}
                style={{ ...btnStyle, width: '100%', justifyContent: 'center', backgroundColor: activeView === 'create' ? THEME.accent : THEME.bgInput, color: activeView === 'create' ? '#fff' : THEME.textMain }}
              >
                <Plus size={16} /> Deploy New Container
              </button>
            </div>
            
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '8px 16px', fontSize: '11px', fontWeight: 'bold', color: THEME.textMuted, textTransform: 'uppercase', letterSpacing: '1px' }}>
              Containers
              <RefreshCw size={12} style={{ cursor: 'pointer' }} onClick={() => loadContainers()} />
            </div>
            
            <div style={{ flex: 1, overflowY: 'auto', padding: '8px' }}>
              {containers.length === 0 ? (
                <div style={{ padding: '16px', textAlign: 'center', color: THEME.textMuted, fontSize: '12px' }}>No containers running.</div>
              ) : containers.map((c: any) => {
                const isRunning = getC(c).state === "running";
                const name = getC(c).names?.[0]?.replace("/", "") || (getC(c).id ? getC(c).id.substring(0,8) : "Unknown");
                const isSelected = activeView === "manage" && selectedId === getC(c).id;
                
                return (
                  <div 
                    key={c.id || Math.random().toString()}
                    onClick={() => { setActiveView("manage"); setSelectedId(getC(c).id); }}
                    style={{ 
                      display: 'flex', 
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      padding: '10px 12px', 
                      cursor: 'pointer', 
                      borderRadius: '4px',
                      marginBottom: '4px',
                      backgroundColor: isSelected ? THEME.bgHover : 'transparent',
                      borderLeft: isSelected ? `3px solid ${THEME.accent}` : '3px solid transparent'
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', overflow: 'hidden', flex: 1 }}>
                      <div style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: isRunning ? THEME.success : THEME.textMuted }} />
                      <span style={{ fontSize: '12px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{name}</span>
                    </div>
                    <button 
                      onClick={(e) => {
                        e.stopPropagation();
                        handleAction("remove", getC(c).id);
                      }}
                      style={{ background: 'transparent', border: 'none', color: THEME.textMuted, cursor: 'pointer', padding: '4px' }}
                      title="Remove Container"
                    >
                      <Trash2 size={12} />
                    </button>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Main Content Area */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', backgroundColor: THEME.bgApp, overflow: 'hidden' }}>
            {activeView === "manage" && currentContainer ? (
              <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
                {/* Header Info & Actions */}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 16px', backgroundColor: THEME.bgPanel, borderBottom: `1px solid ${THEME.border}` }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span style={{ fontSize: '14px', fontWeight: 'bold', marginRight: '16px' }}>{getC(currentContainer).names?.[0]?.replace("/", "") || "Unknown"}</span>
                    {getC(currentContainer).state === 'running' ? (
                      <>
                        <button onClick={() => handleAction("stop", getC(currentContainer).id)} style={{ ...btnStyle, color: THEME.warning, borderColor: THEME.warning }}><Square size={12} fill="currentColor" /> Stop</button>
                        <button onClick={() => handleAction("restart", getC(currentContainer).id)} style={btnStyle}><RefreshCw size={12} /> Restart</button>
                      </>
                    ) : (
                      <button onClick={() => handleAction("start", getC(currentContainer).id)} style={{ ...btnStyle, color: THEME.success, borderColor: THEME.success }}><Play size={12} fill="currentColor" /> Start</button>
                    )}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <button onClick={() => handleAction("remove", getC(currentContainer).id)} style={{ ...btnStyle, color: THEME.error }}><Trash2 size={12} /> Remove</button>
                  </div>
                </div>

                {/* Tabs Nav */}
                <div style={{ display: 'flex', backgroundColor: THEME.bgPanel, borderBottom: `1px solid ${THEME.border}` }}>
                  {[
                    { id: 'logs', label: 'Logs' },
                    { id: 'terminal', label: 'Terminal' },
                    { id: 'settings', label: 'Settings' }
                  ].map(tab => (
                    <button 
                      key={tab.id}
                      onClick={() => setManageTab(tab.id as any)}
                      style={{ 
                        padding: '8px 16px', 
                        background: manageTab === tab.id ? THEME.bgApp : 'transparent', 
                        border: 'none', 
                        borderBottom: manageTab === tab.id ? `2px solid ${THEME.accent}` : '2px solid transparent', 
                        color: manageTab === tab.id ? THEME.accent : THEME.textMuted, 
                        fontSize: '11px', 
                        fontWeight: 'bold', 
                        cursor: 'pointer',
                        textTransform: 'uppercase'
                      }}
                    >
                      {tab.label}
                    </button>
                  ))}
                </div>

                {/* Tab Content */}
                <div style={{ flex: 1, overflow: 'hidden', position: 'relative' }}>
                  {manageTab === "logs" && (
                    <LogsTab 
                      containerId={getC(currentContainer).id} 
                      since={clearedLogsAt[getC(currentContainer).id] || 0}
                      onClear={() => setClearedLogsAt(prev => ({ ...prev, [getC(currentContainer).id]: Math.floor(Date.now() / 1000) }))}
                      key={`logs-${getC(currentContainer).id}`} 
                    />
                  )}
                  {manageTab === "terminal" && <TerminalTab containerId={getC(currentContainer).id} key={`term-${getC(currentContainer).id}`} />}
                  {manageTab === "settings" && (
                    <div style={{ padding: '24px', height: '100%', overflowY: 'auto' }}>
                       <h3 style={{ color: THEME.textMain, marginBottom: '16px' }}>Container Inspect Data</h3>
                       <pre style={{ backgroundColor: THEME.bgPanel, padding: '12px', borderRadius: '4px', border: `1px solid ${THEME.border}`, fontSize: '11px', overflowX: 'auto', color: THEME.textMain }}>
                         {JSON.stringify(currentContainer, null, 2)}
                       </pre>
                    </div>
                  )}
                </div>

                {/* Monochrome small stats footer per user request */}
                {getC(currentContainer).state === 'running' && (
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: '16px', padding: '6px 16px', backgroundColor: THEME.bgPanel, borderTop: `1px solid ${THEME.border}`, fontSize: '11px', color: '#999', fontFamily: 'monospace' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                      <Server size={10} /> 
                      <span style={{ color: '#ccc' }}>CPU: {currentStats.cpu.toFixed(2)}%</span>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                      <Server size={10} /> 
                      <span style={{ color: '#ccc' }}>RAM: {currentStats.ram.toFixed(1)} MB</span>
                    </div>
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
                      <label style={labelStyle}>Image Name *</label>
                      <input list="popular-containers" type="text" value={cImage} onChange={e => setCImage(e.target.value)} style={inputStyle} placeholder="nginx:latest, ubuntu..." />
                      <datalist id="popular-containers">
                        <option value="nginx:latest">Nginx (Web Server)</option>
                        <option value="ubuntu:latest">Ubuntu Linux</option>
                        <option value="alpine:latest">Alpine Linux</option>
                        <option value="redis:latest">Redis</option>
                        <option value="postgres:latest">PostgreSQL</option>
                        <option value="mysql:latest">MySQL</option>
                        <option value="mongo:latest">MongoDB</option>
                        <option value="node:latest">Node.js</option>
                        <option value="python:latest">Python</option>
                        <option value="httpd:latest">Apache HTTP Server</option>
                      </datalist>
                    </div>
                    <div>
                      <label style={labelStyle}>Container Name (Optional)</label>
                      <input type="text" value={cName} onChange={e => setCName(e.target.value)} style={inputStyle} placeholder="my-nginx-app" />
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
