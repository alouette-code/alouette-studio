import { useState } from "react";
import { 
  ChevronDown, 
  ChevronRight, 
  Layers, 
  Globe,
  HardDrive
} from "lucide-react";
import { Project, ProcessState, ResourceHistory, ChildProcessInfo } from "../types";

interface ProcessManagerProps {
  projects: Project[];
  activeProjectId: string;
  setActiveProjectId: (id: string) => void;
  projectStates: { [id: string]: ProcessState };
  resourceHistory: ResourceHistory;
  handleStartProject: (id: string) => void;
  handleStopProject: (id: string) => void;
  forceKillProcess: (pid: number) => Promise<void>;
  triggerConfirm: (message: string, onConfirm: () => void) => void;
  triggerToast: (message: string, type: "success" | "error" | "info") => void;
}

export default function ProcessManager({
  projects,
  activeProjectId,
  setActiveProjectId,
  projectStates,
  resourceHistory,
  handleStartProject,
  handleStopProject,
  forceKillProcess,
  triggerConfirm,
  triggerToast
}: ProcessManagerProps) {
  // Track expanded state for projects (to show child processes)
  const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({});
  // Track expanded state for individual processes (to show ports, threads, and maps)
  const [expandedProcesses, setExpandedProcesses] = useState<Record<number, boolean>>({});

  const toggleProjectExpand = (projectId: string) => {
    setExpandedProjects(prev => ({
      ...prev,
      [projectId]: !prev[projectId]
    }));
  };

  const toggleProcessExpand = (pid: number) => {
    setExpandedProcesses(prev => ({
      ...prev,
      [pid]: !prev[pid]
    }));
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0.0 MB";
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="lower-panel-manager" style={{ padding: '12px', display: 'flex', flexDirection: 'column', gap: '8px', height: '100%', overflow: 'hidden' }}>
      <div className="panel-header-row" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid var(--border-primary)', paddingBottom: '8px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <h3 style={{ fontSize: '11px', margin: 0, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Process Tree</h3>
        </div>
        <span className="sys-badge" style={{ fontSize: '9px', padding: '2px 6px', borderRadius: '4px', backgroundColor: 'var(--bg-secondary)', color: 'var(--text-muted)', border: '1px solid var(--border-primary)' }}>OS: Linux</span>
      </div>

      <div className="manager-table-wrapper" style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden' }}>
        <table className="manager-table" style={{ width: '100%', borderCollapse: 'collapse', fontSize: '11px' }}>
          <thead>
            <tr style={{ textAlign: 'left', borderBottom: '1px solid var(--border-primary)', color: 'var(--text-muted)' }}>
              <th style={{ padding: '6px 8px' }}>Process Name / PID</th>
              <th style={{ padding: '6px 8px' }}>Status</th>
              <th style={{ padding: '6px 8px' }}>CPU</th>
              <th style={{ padding: '6px 8px' }}>Memory</th>
              <th style={{ padding: '6px 8px', textAlign: 'right' }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {projects.map((p) => {
              const state = projectStates[p.id] || { type: "Stopped" };
              const history = resourceHistory[p.id];
              const cpu = history && history.cpu.length > 0 ? history.cpu[history.cpu.length - 1] : 0;
              const ram = history && history.ram.length > 0 ? history.ram[history.ram.length - 1] : 0;
              const isProjectActive = p.id === activeProjectId;
              const childProcesses: ChildProcessInfo[] = history?.processes || [];
              const isExpanded = !!expandedProjects[p.id];

              return (
                <>
                  {/* Project Main Row */}
                  <tr key={p.id} className={isProjectActive ? "active-row" : ""} style={{ borderBottom: '1px solid var(--border-secondary)', transition: 'background 0.2s' }}>
                    <td style={{ padding: '8px', display: 'flex', alignItems: 'center', gap: '4px', fontWeight: 600 }}>
                      {state.type === "Running" || childProcesses.length > 0 ? (
                        <button 
                          onClick={() => toggleProjectExpand(p.id)} 
                          style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', padding: 0, display: 'flex', alignItems: 'center' }}
                        >
                          {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                        </button>
                      ) : (
                        <span style={{ width: '14px' }} />
                      )}
                      <span onClick={() => setActiveProjectId(p.id)} style={{ cursor: 'pointer', color: 'var(--text-primary)' }}>
                        {p.name}
                      </span>
                    </td>
                    <td style={{ padding: '8px' }}>
                      <span className={`status-pill status-${(state.type === "Running" || childProcesses.length > 0) ? "running" : "stopped"}`} style={{ fontSize: '9px', padding: '1px 5px', borderRadius: '3px', fontWeight: 500 }}>
                        {state.type === "Running" || childProcesses.length > 0 ? "Running" : "Stopped"}
                      </span>
                    </td>
                    <td className="mono" style={{ padding: '8px' }}>
                      {state.type === "Running" || childProcesses.length > 0 ? `${cpu.toFixed(1)}%` : "0.0%"}
                    </td>
                    <td className="mono" style={{ padding: '8px' }}>
                      {state.type === "Running" || childProcesses.length > 0 ? `${ram.toFixed(1)} MB` : "0.0 MB"}
                    </td>
                    <td style={{ padding: '8px', textAlign: 'right' }}>
                      <div style={{ display: "flex", gap: "6px", justifyContent: 'flex-end' }}>
                        {state.type === "Running" ? (
                          <button
                            className="btn btn-danger btn-xs"
                            style={{ padding: '2px 8px', fontSize: '9px', borderRadius: '3px' }}
                            onClick={() => {
                              setActiveProjectId(p.id);
                              handleStopProject(p.id);
                            }}
                          >
                            Kill All
                          </button>
                        ) : (
                          <button
                            className="btn btn-primary btn-xs"
                            style={{ padding: '2px 8px', fontSize: '9px', borderRadius: '3px' }}
                            onClick={() => {
                              setActiveProjectId(p.id);
                              handleStartProject(p.id);
                            }}
                          >
                            Run
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>

                  {/* Child Processes View (Nested Tree) */}
                  {(state.type === "Running" || childProcesses.length > 0) && isExpanded && (
                    <>
                      {childProcesses.length === 0 ? (
                        <tr>
                          <td colSpan={5} style={{ padding: '6px 8px 6px 24px', color: 'var(--text-muted)', fontStyle: 'italic' }}>
                            Tracing processes in background...
                          </td>
                        </tr>
                      ) : (
                        childProcesses.map((cp) => {
                          const isProcessExpanded = !!expandedProcesses[cp.pid];
                          const hasPorts = cp.ports && cp.ports.length > 0;
                          
                          // Determine status display styling
                          let statusColor = 'var(--text-muted)';
                          if (cp.status === "Running") statusColor = '#10b981'; // Green
                          else if (cp.status === "Sleeping") statusColor = '#3a86ff'; // Blue
                          else if (cp.status === "Stopped") statusColor = '#f59e0b'; // Amber

                          return (
                            <>
                              {/* Sub-process row */}
                              <tr key={`cp-${cp.pid}`} style={{ backgroundColor: 'rgba(255, 255, 255, 0.02)', borderBottom: '1px dashed var(--border-secondary)' }}>
                                <td style={{ padding: '6px 8px 6px 20px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                                  <div style={{ width: '1px', height: '14px', backgroundColor: 'var(--border-primary)', marginRight: '4px' }} />
                                  <button
                                    onClick={() => toggleProcessExpand(cp.pid)}
                                    style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', padding: 0, display: 'flex', alignItems: 'center' }}
                                  >
                                    {isProcessExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                                  </button>
                                  <span style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)', fontSize: '10px', marginRight: '4px' }}>
                                    [{cp.pid}]
                                  </span>
                                  <span style={{ fontWeight: 500, color: 'var(--text-primary)' }} title={cp.cmd}>
                                    {cp.name}
                                  </span>
                                </td>
                                <td style={{ padding: '6px 8px' }}>
                                  <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', color: statusColor, fontWeight: 500, fontSize: '9px' }}>
                                    <span style={{ width: '5px', height: '5px', borderRadius: '50%', backgroundColor: statusColor }} />
                                    {cp.status}
                                  </span>
                                </td>
                                <td className="mono" style={{ padding: '6px 8px', color: 'var(--text-secondary)' }}>
                                  {cp.cpu_percentage.toFixed(1)}%
                                </td>
                                <td className="mono" style={{ padding: '6px 8px', color: 'var(--text-secondary)' }}>
                                  {formatBytes(cp.ram_bytes)}
                                </td>
                                <td style={{ padding: '6px 8px', textAlign: 'right' }}>
                                  <button
                                    className="btn btn-secondary btn-xs"
                                    style={{ padding: '1px 6px', fontSize: '9px', borderRadius: '3px', border: '1px solid var(--border-primary)' }}
                                    onClick={() => {
                                      triggerConfirm(`Are you sure you want to force kill PID ${cp.pid} (${cp.name})?`, async () => {
                                        try {
                                          await forceKillProcess(cp.pid);
                                          triggerToast(`Process ${cp.pid} terminated successfully.`, "success");
                                        } catch (e: any) {
                                          triggerToast(`Failed to kill process: ${e}`, "error");
                                        }
                                      });
                                    }}
                                  >
                                    Kill
                                  </button>
                                </td>
                              </tr>

                              {/* Nested Deep Dive Details Drawer */}
                              {isProcessExpanded && (
                                <tr>
                                  <td colSpan={5} style={{ padding: '8px 12px 10px 40px', backgroundColor: 'rgba(0, 0, 0, 0.15)' }}>
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', borderLeft: '2px solid var(--color-accent)', paddingLeft: '10px' }}>
                                      {/* Command & CWD */}
                                      <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>
                                        <div style={{ marginBottom: '2px' }}>
                                          <strong style={{ color: 'var(--text-muted)' }}>CWD: </strong> 
                                          <span style={{ fontFamily: 'var(--font-mono)' }}>{cp.cwd || 'System Root'}</span>
                                        </div>
                                        <div>
                                          <strong style={{ color: 'var(--text-muted)' }}>Command: </strong> 
                                          <span style={{ fontFamily: 'var(--font-mono)', wordBreak: 'break-all' }}>{cp.cmd}</span>
                                        </div>
                                      </div>

                                      {/* Network Ports & Threads */}
                                      <div style={{ display: 'flex', gap: '20px', margin: '4px 0' }}>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                                          <Globe size={11} style={{ color: hasPorts ? '#10b981' : 'var(--text-muted)' }} />
                                          <span style={{ fontWeight: 600 }}>Ports: </span>
                                          {hasPorts ? (
                                            cp.ports.map(port => (
                                              <span key={port} style={{ backgroundColor: 'rgba(16, 185, 129, 0.15)', color: '#10b981', padding: '1px 4px', borderRadius: '3px', fontFamily: 'var(--font-mono)', fontWeight: 600, fontSize: '10px' }}>
                                                :{port}
                                              </span>
                                            ))
                                          ) : (
                                            <span style={{ color: 'var(--text-muted)', fontSize: '10px' }}>No active ports</span>
                                          )}
                                        </div>
                                        
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                                          <Layers size={11} style={{ color: 'var(--color-accent)' }} />
                                          <span style={{ fontWeight: 600 }}>Threads: </span>
                                          <span style={{ fontFamily: 'var(--font-mono)', fontSize: '10px' }}>{cp.thread_count} active</span>
                                        </div>
                                      </div>

                                      {/* Loaded Modules / Assets (Dynamic engine maps) */}
                                      <div style={{ marginTop: '2px' }}>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '4px' }}>
                                          <HardDrive size={11} style={{ color: 'var(--text-muted)' }} />
                                          <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>Loaded Libraries & Assets</span>
                                        </div>
                                        {cp.loaded_modules && cp.loaded_modules.length > 0 ? (
                                          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', maxHeight: '60px', overflowY: 'auto', padding: '4px', backgroundColor: 'var(--bg-secondary)', borderRadius: '4px', border: '1px solid var(--border-primary)' }}>
                                            {cp.loaded_modules.map((mod, idx) => {
                                              const filename = mod.split('/').pop() || mod;
                                              return (
                                                <span key={idx} title={mod} style={{ backgroundColor: 'rgba(255, 255, 255, 0.05)', color: 'var(--text-muted)', padding: '1px 4px', borderRadius: '2px', fontSize: '9px', fontFamily: 'var(--font-mono)' }}>
                                                  {filename}
                                                </span>
                                              );
                                            })}
                                          </div>
                                        ) : (
                                          <span style={{ color: 'var(--text-muted)', fontSize: '10px', fontStyle: 'italic' }}>No system maps loaded</span>
                                        )}
                                      </div>
                                    </div>
                                  </td>
                                </tr>
                              )}
                            </>
                          );
                        })
                      )}
                    </>
                  )}
                </>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
