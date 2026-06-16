import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Play, Power, FolderOpen, Cpu, HardDrive, Network, Settings2, Link as LinkIcon, Download, Trash2, Plus } from "lucide-react";

export default function LocalAiManager() {
  const [selectedEngine, setSelectedEngine] = useState("ollama");
  const [hardwareTarget, setHardwareTarget] = useState("gpu");
  const [ramLimit, setRamLimit] = useState(8);
  const [cpuThreads, setCpuThreads] = useState(4);
  const [port, setPort] = useState(11434);
  const [apiHost, setApiHost] = useState("127.0.0.1");
  const [apiRoute, setApiRoute] = useState("/v1");
  const [modelName, setModelName] = useState("");
  const [sourcePath, setSourcePath] = useState("");
  
  // Dashboard states
  const [activeEngines, setActiveEngines] = useState<any[]>([]);
  const [savedConfigs, setSavedConfigs] = useState<any[]>([]);

  const engines = [
    { id: "ollama", name: "Ollama", desc: "Lightweight, native inference for GGUF models.", sourceLabel: "Model Name / Tag", sourcePlaceholder: "e.g. llama3:8b", isInstalled: true, hardwareLocked: null, needsBrowse: false },
    { id: "llamacpp", name: "Llama.cpp (Candle Native)", desc: "Pure Rust inference engine for GGUF models.", sourceLabel: "Model Source Path (.gguf)", sourcePlaceholder: "/path/to/model.gguf", isInstalled: true, hardwareLocked: null, needsBrowse: true },
    { id: "onnx", name: "ONNX Runtime (Native)", desc: "Runs ONNX models with hardware acceleration.", sourceLabel: "ONNX Model Folder", sourcePlaceholder: "/path/to/onnx_model", isInstalled: true, hardwareLocked: null, needsBrowse: true }
  ];

  const currentEngineInfo = engines.find(e => e.id === selectedEngine) || engines[0];
  const fullEndpointUrl = `http://${apiHost}:${port}${apiRoute}/chat/completions`;

  const fetchSavedConfigs = async () => {
    try {
      const configs = await invoke<any[]>("load_ai_settings");
      setSavedConfigs(configs || []);
    } catch (err) {
      console.error("Failed to load saved configs:", err);
    }
  };

  // 1. Load Settings on Mount
  useEffect(() => {
    fetchSavedConfigs();
  }, []);

  // Poll active engines every 2 seconds
  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const status = await invoke<any[]>("get_ai_engine_status");
        setActiveEngines(status || []);
      } catch (err) {
        console.error("Failed to fetch engine status:", err);
      }
    };

    fetchStatus();
    const interval = setInterval(fetchStatus, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleEngineChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newEngineId = e.target.value;
    setSelectedEngine(newEngineId);
    const engineInfo = engines.find(eng => eng.id === newEngineId);
    if (engineInfo && engineInfo.hardwareLocked) {
      setHardwareTarget(engineInfo.hardwareLocked);
    }
  };

  const handleBrowse = async () => {
    try {
      const isFolder = currentEngineInfo.sourceLabel.toLowerCase().includes("folder");
      const path = await invoke<string | null>(isFolder ? "open_folder_dialog" : "open_file_dialog");
      if (path) {
        setSourcePath(path);
      }
    } catch (err) {
      console.error("Browse failed:", err);
    }
  };

  const handleAddConfig = async () => {
    try {
      await invoke("save_ai_settings", {
        config: {
          engine_id: selectedEngine,
          model_name: modelName,
          source_path: sourcePath,
          hardware_target: hardwareTarget,
          ram_limit_gb: ramLimit,
          cpu_threads: cpuThreads,
          api_host: apiHost,
          port: port,
          api_route: apiRoute
        }
      });
      fetchSavedConfigs(); // Refresh list
    } catch (err) {
      console.error("Failed to save config:", err);
      alert(err);
    }
  };

  const handleStartEngine = async (config: any) => {
    try {
      await invoke("start_ai_engine", { config });
    } catch (err) {
      console.error("Failed to start engine:", err);
      alert(err);
    }
  };

  const handleStopEngine = async (engineId: string) => {
    try {
      await invoke("stop_ai_engine", { engineId: engineId });
    } catch (err) {
      console.error("Failed to stop engine:", err);
    }
  };

  const handleDeleteConfig = async (engineId: string, mName: string) => {
    try {
      await invoke("delete_ai_setting", { engineId: engineId, modelName: mName || null });
      fetchSavedConfigs(); // Refresh list
    } catch (err) {
      console.error("Failed to delete config:", err);
    }
  };

  return (
    <div className="local-ai-manager" style={{ display: 'flex', flexDirection: 'column', height: '100%', backgroundColor: 'var(--bg-primary)', color: 'var(--text-primary)', overflowY: 'auto' }}>
      
      {/* Header */}
      <div style={{ padding: '20px 24px', borderBottom: '1px solid var(--border-primary)', display: 'flex', alignItems: 'center', gap: '12px', backgroundColor: 'var(--bg-secondary)' }}>
        <Settings2 size={20} color="var(--accent)" />
        <h2 style={{ fontSize: '16px', margin: 0, fontWeight: 600 }}>Local AI Engine Configuration</h2>
      </div>

      <div style={{ padding: '24px', maxWidth: '800px', display: 'flex', flexDirection: 'column', gap: '32px' }}>
        
        {/* Section: Core Config */}
        <section>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '16px', color: 'var(--text-secondary)' }}>
            <Cpu size={16} />
            <h3 style={{ fontSize: '14px', margin: 0, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Core Engine & Hardware</h3>
          </div>
          
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '24px' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <label style={{ fontSize: '13px', fontWeight: 500 }}>Execution Engine</label>
              <select 
                value={selectedEngine} 
                onChange={handleEngineChange}
                style={{ padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', cursor: 'pointer', fontSize: '14px' }}
              >
                {engines.map(e => <option key={e.id} value={e.id}>{e.name}</option>)}
              </select>
              <div style={{ fontSize: '12px', color: 'var(--text-muted)' }}>{currentEngineInfo.desc}</div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <label style={{ fontSize: '13px', fontWeight: 500 }}>
                Hardware Acceleration 
                {currentEngineInfo.hardwareLocked && <span style={{ color: 'var(--warning)', marginLeft: '8px', fontSize: '11px', fontStyle: 'italic' }}>(Locked by Engine)</span>}
              </label>
              <div style={{ display: 'flex', backgroundColor: 'var(--bg-tertiary)', borderRadius: '6px', border: '1px solid var(--border-primary)', overflow: 'hidden', opacity: currentEngineInfo.hardwareLocked ? 0.7 : 1 }}>
                {['cpu', 'igpu', 'gpu'].map(hw => (
                  <button 
                    key={hw}
                    disabled={currentEngineInfo.hardwareLocked !== null && currentEngineInfo.hardwareLocked !== hw}
                    onClick={() => setHardwareTarget(hw)}
                    style={{ 
                      flex: 1, padding: '8px 12px', border: 'none', 
                      backgroundColor: hardwareTarget === hw ? 'var(--accent)' : 'transparent', 
                      color: hardwareTarget === hw ? '#fff' : 'var(--text-primary)', 
                      cursor: currentEngineInfo.hardwareLocked ? 'not-allowed' : 'pointer', 
                      fontWeight: hardwareTarget === hw ? 600 : 400,
                      textTransform: 'uppercase', fontSize: '13px', transition: 'all 0.15s ease'
                    }}>
                    {hw}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* Section: Model Identity */}
        <section>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '16px', color: 'var(--text-secondary)' }}>
            <HardDrive size={16} />
            <h3 style={{ fontSize: '14px', margin: 0, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Model & Source</h3>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 2fr', gap: '24px' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <label style={{ fontSize: '13px', fontWeight: 500 }}>Display Name</label>
              <input 
                type="text" 
                placeholder="e.g. Llama-3-8B" 
                value={modelName} 
                onChange={(e) => setModelName(e.target.value)} 
                style={{ padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} 
              />
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <label style={{ fontSize: '13px', fontWeight: 500 }}>{currentEngineInfo.sourceLabel}</label>
              <div style={{ display: 'flex', gap: '8px' }}>
                <input 
                  type="text" 
                  value={sourcePath}
                  onChange={(e) => setSourcePath(e.target.value)}
                  placeholder={currentEngineInfo.sourcePlaceholder} 
                  style={{ flex: 1, padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} 
                />
                {currentEngineInfo.needsBrowse && (
                  <button onClick={handleBrowse} style={{ padding: '8px 16px', borderRadius: '6px', backgroundColor: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '6px', fontSize: '14px', fontWeight: 500, transition: 'background 0.2s' }}>
                    <FolderOpen size={16} /> Browse
                  </button>
                )}
              </div>
            </div>
          </div>
        </section>

        {/* Section: Resource Limits & Network */}
        <section>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '16px', color: 'var(--text-secondary)' }}>
            <Network size={16} />
            <h3 style={{ fontSize: '14px', margin: 0, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Resources & Network</h3>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
            <div style={{ display: 'flex', gap: '32px' }}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', width: '120px' }}>
                <label style={{ fontSize: '13px', fontWeight: 500 }}>RAM Limit (GB)</label>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <input type="number" min="2" max="128" value={ramLimit} onChange={(e) => setRamLimit(Number(e.target.value))} style={{ width: '100%', padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} />
                </div>
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', width: '120px' }}>
                <label style={{ fontSize: '13px', fontWeight: 500 }}>CPU Threads</label>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <input type="number" min="1" max="64" value={cpuThreads} onChange={(e) => setCpuThreads(Number(e.target.value))} style={{ width: '100%', padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} />
                </div>
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                <label style={{ fontSize: '13px', fontWeight: 500 }}>API Host</label>
                <input type="text" value={apiHost} onChange={(e) => setApiHost(e.target.value)} style={{ width: '100%', padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', width: '100px' }}>
                <label style={{ fontSize: '13px', fontWeight: 500 }}>Port</label>
                <input type="number" value={port} onChange={(e) => setPort(Number(e.target.value))} style={{ width: '100%', padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', width: '100px' }}>
                <label style={{ fontSize: '13px', fontWeight: 500 }}>API Route</label>
                <input type="text" value={apiRoute} onChange={(e) => setApiRoute(e.target.value)} style={{ width: '100%', padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} />
              </div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <label style={{ fontSize: '13px', fontWeight: 500, display: 'flex', alignItems: 'center', gap: '6px' }}>
                <LinkIcon size={14} color="var(--accent)" />
                API Gateway Endpoint
              </label>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <input 
                  type="text" 
                  readOnly 
                  value={fullEndpointUrl} 
                  style={{ flex: 1, padding: '10px 14px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-primary)', color: 'var(--success)', outline: 'none', fontSize: '14px', fontFamily: 'monospace' }} 
                />
                <button 
                  onClick={() => navigator.clipboard.writeText(fullEndpointUrl)}
                  style={{ padding: '10px 16px', borderRadius: '6px', backgroundColor: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px', fontWeight: 500, transition: 'background 0.2s' }}>
                  Copy URL
                </button>
              </div>
            </div>
          </div>
        </section>

        {/* Action Button */}
        <div style={{ marginTop: '16px', display: 'flex', alignItems: 'center', gap: '16px' }}>
          {currentEngineInfo.isInstalled ? (
            <button 
              onClick={handleAddConfig}
              style={{ 
                display: 'flex', alignItems: 'center', gap: '8px',
                padding: '10px 24px', borderRadius: '6px', 
                backgroundColor: 'var(--accent)', 
                color: '#fff', border: 'none', 
                cursor: 'pointer', fontWeight: 600, fontSize: '14px',
                transition: 'all 0.2s ease', boxShadow: '0 2px 8px rgba(0,0,0,0.1)'
              }}
            >
              <Plus size={16} fill="currentColor" /> Add to Dashboard
            </button>
          ) : (
            <button 
              style={{ 
                display: 'flex', alignItems: 'center', gap: '8px',
                padding: '10px 24px', borderRadius: '6px', 
                backgroundColor: 'var(--warning)', 
                color: '#000', border: 'none', cursor: 'pointer', fontWeight: 600, fontSize: '14px',
                transition: 'all 0.2s ease', boxShadow: '0 2px 8px rgba(0,0,0,0.1)'
              }}
            >
              <Download size={16} color="#000" /> Install Engine
            </button>
          )}
        </div>

        <hr style={{ border: 'none', borderTop: '1px solid var(--border-primary)', margin: '16px 0' }} />

        {/* Section: Active Engines Dashboard */}
        <section>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '16px' }}>
            <span style={{ width: '10px', height: '10px', borderRadius: '50%', backgroundColor: 'var(--text-muted)', display: 'inline-block' }}></span>
            <h3 style={{ fontSize: '14px', margin: 0, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-primary)' }}>Saved Models Registry</h3>
            <span style={{ marginLeft: 'auto', fontSize: '11px', fontWeight: 600, color: 'var(--text-secondary)', backgroundColor: 'var(--bg-tertiary)', padding: '4px 10px', borderRadius: '12px', border: '1px solid var(--border-primary)' }}>
              {activeEngines.length} Running / {savedConfigs.length} Total
            </span>
          </div>

          {savedConfigs.length === 0 ? (
            <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)', backgroundColor: 'transparent', borderRadius: '8px', border: '1px dashed var(--border-primary)', fontSize: '13px' }}>
              No models in registry. Fill out the config above and click "Add to Dashboard".
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
              {savedConfigs.map((config, idx) => {
                const isRunning = activeEngines.some(e => e.engine_id === config.engine_id && e.model_name === config.model_name);
                
                return (
                  <div key={idx} style={{ 
                    display: 'flex', alignItems: 'center', justifyContent: 'space-between', 
                    padding: '14px 18px', borderRadius: '10px', backgroundColor: 'var(--bg-secondary)', 
                    border: '1px solid var(--border-primary)', transition: 'all 0.2s ease',
                    boxShadow: '0 2px 8px rgba(0,0,0,0.05)'
                  }}>
                    
                    {/* Left Side: Model Identity */}
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span style={{ 
                          width: '8px', height: '8px', borderRadius: '50%', 
                          backgroundColor: isRunning ? 'var(--success)' : 'var(--text-muted)', 
                          display: 'inline-block', 
                          boxShadow: isRunning ? '0 0 8px var(--success)' : 'none' 
                        }}></span>
                        <strong style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>{config.model_name || config.engine_id}</strong>
                        <span style={{ fontSize: '10px', padding: '2px 6px', borderRadius: '4px', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-secondary)', border: '1px solid var(--border-primary)', textTransform: 'uppercase' }}>
                          {engines.find(e => e.id === config.engine_id)?.name || config.engine_id}
                        </span>
                        <span style={{ fontSize: '10px', fontWeight: 600, color: isRunning ? 'var(--success)' : 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                          [{isRunning ? 'Running' : 'Stopped'}]
                        </span>
                      </div>
                    </div>
                    
                    {/* Right Side: Resources & Action */}
                    <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
                      <div style={{ display: 'flex', gap: '16px', fontSize: '12px', color: 'var(--text-secondary)', fontWeight: 500 }}>
                        <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }} title="Hardware Target">
                          <Cpu size={14} color="var(--text-muted)" />
                          <span style={{ textTransform: 'uppercase' }}>{config.hardware_target}</span>
                        </span>
                        <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }} title="RAM Limit">
                          <HardDrive size={14} color="var(--text-muted)" />
                          {config.ram_limit_gb}GB
                        </span>
                        <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }} title="CPU Threads">
                          <Settings2 size={14} color="var(--text-muted)" />
                          {config.cpu_threads}T
                        </span>
                      </div>
                      
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        {isRunning ? (
                          <button 
                            onClick={() => handleStopEngine(config.engine_id)}
                            style={{ 
                              padding: '6px 14px', borderRadius: '6px', border: '1px solid rgba(239, 68, 68, 0.3)',
                              backgroundColor: 'transparent', color: '#ef4444', 
                              cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '6px', 
                              fontSize: '12px', fontWeight: 600, transition: 'all 0.2s ease' 
                            }}
                            onMouseOver={(e) => { e.currentTarget.style.backgroundColor = '#ef4444'; e.currentTarget.style.color = '#fff'; }}
                            onMouseOut={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = '#ef4444'; }}
                          >
                            <Power size={12} strokeWidth={3} /> Stop
                          </button>
                        ) : (
                          <button 
                            onClick={() => handleStartEngine(config)}
                            style={{ 
                              padding: '6px 14px', borderRadius: '6px', border: '1px solid rgba(16, 185, 129, 0.3)',
                              backgroundColor: 'transparent', color: 'var(--success)', 
                              cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '6px', 
                              fontSize: '12px', fontWeight: 600, transition: 'all 0.2s ease' 
                            }}
                            onMouseOver={(e) => { e.currentTarget.style.backgroundColor = 'var(--success)'; e.currentTarget.style.color = '#fff'; }}
                            onMouseOut={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = 'var(--success)'; }}
                          >
                            <Play size={12} fill="currentColor" /> Start
                          </button>
                        )}

                        {!isRunning && (
                          <button 
                            onClick={() => handleDeleteConfig(config.engine_id, config.model_name)}
                            style={{ 
                              padding: '6px', borderRadius: '6px', border: '1px solid var(--border-primary)',
                              backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-muted)', 
                              cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center',
                              transition: 'all 0.2s ease' 
                            }}
                            title="Delete Configuration"
                            onMouseOver={(e) => { e.currentTarget.style.color = '#ef4444'; e.currentTarget.style.borderColor = 'rgba(239, 68, 68, 0.5)'; }}
                            onMouseOut={(e) => { e.currentTarget.style.color = 'var(--text-muted)'; e.currentTarget.style.borderColor = 'var(--border-primary)'; }}
                          >
                            <Trash2 size={14} />
                          </button>
                        )}
                      </div>

                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </section>

      </div>
    </div>
  );
}
