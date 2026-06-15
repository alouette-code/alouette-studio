import React, { useState } from "react";
import { Play, Square, FolderOpen, Cpu, HardDrive, Network, Settings2, Link as LinkIcon, Download } from "lucide-react";

export default function LocalAiManager() {
  const [selectedEngine, setSelectedEngine] = useState("ollama");
  const [hardwareTarget, setHardwareTarget] = useState("gpu");
  const [ramLimit, setRamLimit] = useState(8);
  const [cpuThreads, setCpuThreads] = useState(4);
  const [port, setPort] = useState(11434);
  const [apiHost, setApiHost] = useState("127.0.0.1");
  const [apiRoute, setApiRoute] = useState("/v1");
  const [modelName, setModelName] = useState("");
  const [isRunning, setIsRunning] = useState(false);

  const engines = [
    { id: "ollama", name: "Ollama", desc: "Lightweight, native inference for GGUF models.", sourceLabel: "Model Name / Tag", sourcePlaceholder: "e.g. llama3:8b", isInstalled: true, hardwareLocked: null, needsBrowse: false },
    { id: "llamacpp", name: "llama.cpp Server", desc: "Highly optimized C++ inference engine for GGUF/GGML.", sourceLabel: "Model Source Path (.gguf)", sourcePlaceholder: "/path/to/model.gguf", isInstalled: false, hardwareLocked: null, needsBrowse: true },
    { id: "onnx", name: "ONNX Runtime", desc: "Runs ONNX models with hardware acceleration.", sourceLabel: "ONNX Model Folder", sourcePlaceholder: "/path/to/onnx_model", isInstalled: false, hardwareLocked: null, needsBrowse: true },
    { id: "tensorrt", name: "TensorRT-LLM", desc: "NVIDIA's high-performance engine for .engine files.", sourceLabel: "TensorRT Engine File", sourcePlaceholder: "/path/to/model.engine", isInstalled: false, hardwareLocked: "gpu", needsBrowse: true },
    { id: "vllm", name: "vLLM", desc: "High-throughput serving for Safetensors, PyTorch (.bin, .pt).", sourceLabel: "HuggingFace Format Folder", sourcePlaceholder: "/path/to/safetensors_folder", isInstalled: false, hardwareLocked: "gpu", needsBrowse: true },
    { id: "koboldcpp", name: "Koboldcpp", desc: "Easy-to-use llama.cpp fork with a built-in API.", sourceLabel: "Model Source Path (.gguf)", sourcePlaceholder: "/path/to/model.gguf", isInstalled: false, hardwareLocked: null, needsBrowse: true },
    { id: "exllamav2", name: "ExLlamaV2", desc: "Extremely fast inference for EXL2 quantized models.", sourceLabel: "EXL2 Model Folder", sourcePlaceholder: "/path/to/exl2_folder", isInstalled: false, hardwareLocked: "gpu", needsBrowse: true },
    { id: "mlx", name: "MLX (Apple Silicon)", desc: "Apple Silicon optimized ML array framework.", sourceLabel: "MLX Model Folder", sourcePlaceholder: "/path/to/mlx_folder", isInstalled: false, hardwareLocked: "gpu", needsBrowse: true } // MLX uses GPU/Metal Unified Memory
  ];

  const currentEngineInfo = engines.find(e => e.id === selectedEngine) || engines[0];
  const fullEndpointUrl = `http://${apiHost}:${port}${apiRoute}/chat/completions`;

  const handleEngineChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newEngineId = e.target.value;
    setSelectedEngine(newEngineId);
    const engineInfo = engines.find(eng => eng.id === newEngineId);
    if (engineInfo && engineInfo.hardwareLocked) {
      setHardwareTarget(engineInfo.hardwareLocked);
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
                  placeholder={currentEngineInfo.sourcePlaceholder} 
                  style={{ flex: 1, padding: '8px 12px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px' }} 
                />
                {currentEngineInfo.needsBrowse && (
                  <button style={{ padding: '8px 16px', borderRadius: '6px', backgroundColor: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '6px', fontSize: '14px', fontWeight: 500, transition: 'background 0.2s' }}>
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
                Full API Endpoint URL
              </label>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <input 
                  type="text" 
                  readOnly 
                  value={fullEndpointUrl} 
                  style={{ flex: 1, padding: '10px 14px', borderRadius: '6px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-primary)', color: 'var(--text-primary)', outline: 'none', fontSize: '14px', fontFamily: 'monospace', color: 'var(--success)' }} 
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

        {/* Action Button & Runtime Check */}
        <div style={{ marginTop: '16px', display: 'flex', alignItems: 'center', gap: '16px' }}>
          {currentEngineInfo.isInstalled ? (
            <button 
              onClick={() => setIsRunning(!isRunning)}
              style={{ 
                display: 'flex', alignItems: 'center', gap: '8px',
                padding: '10px 24px', borderRadius: '6px', 
                backgroundColor: isRunning ? 'var(--error)' : 'var(--accent)', 
                color: '#fff', border: 'none', cursor: 'pointer', fontWeight: 600, fontSize: '14px',
                transition: 'all 0.2s ease', boxShadow: '0 2px 8px rgba(0,0,0,0.1)'
              }}
            >
              {isRunning ? <><Square size={16} fill="currentColor" /> Stop Engine</> : <><Play size={16} fill="currentColor" /> Start Engine</>}
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

          <span style={{ color: 'var(--text-muted)', fontSize: '13px', fontStyle: 'italic' }}>
            {!currentEngineInfo.isInstalled 
              ? `${currentEngineInfo.name} binaries not found. Click to install.` 
              : isRunning 
                ? `Running ${currentEngineInfo.name} on ${fullEndpointUrl}.` 
                : "Ready. Core binaries verified."}
          </span>
        </div>

      </div>
    </div>
  );
}
