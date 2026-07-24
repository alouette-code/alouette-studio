import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Play, Square, Settings2 } from 'lucide-react';
import Editor from '@monaco-editor/react';
import { InspectionConfig } from '../types';

const getMonacoLanguage = (lang: string) => {
    switch (lang) {
        case 'node': return 'javascript';
        case 'c':
        case 'cpp': return 'cpp';
        case 'bash': return 'shell';
        case 'php': return 'php';
        case 'ruby': return 'ruby';
        case 'java': return 'java';
        case 'go': return 'go';
        case 'rust': return 'rust';
        case 'python': return 'python';
        default: return 'plaintext';
    }
};

interface ControlPanelProps {
    isActive: boolean;
    onStart: (config: InspectionConfig) => void;
    onStop: () => void;
}

export function ControlPanel({ isActive, onStart, onStop }: ControlPanelProps) {
    const [image, setImage] = useState('redis:alpine');
    const [initialRam, setInitialRam] = useState(512);
    const [envVars, setEnvVars] = useState('');
    const [ports, setPorts] = useState('');
    const [volumes, setVolumes] = useState('');
    const [network, setNetwork] = useState('');
    const [cmd, setCmd] = useState('');
    const [showAdvanced, setShowAdvanced] = useState(false);
    const [targetMode, setTargetMode] = useState<'docker' | 'snippet' | 'executable'>('docker');
    const [snippetLang, setSnippetLang] = useState('python');
    const [snippetCode, setSnippetCode] = useState('print("Hello World")');
    const [executablePath, setExecutablePath] = useState('');

    const [availableImages, setAvailableImages] = useState<string[]>([
        'redis:alpine', 'nginx:latest', 'node:18-alpine', 'python:3.9-slim', 'postgres:15-alpine'
    ]);

    const handleSelectFile = async () => {
        try {
            const result = await invoke('open_file_dialog');
            if (result) setExecutablePath(result as string);
        } catch (e) {
            console.error(e);
        }
    };

    useEffect(() => {
        // Auto-ensure Docker daemon is started on mount
        invoke('docker_ensure_started')
            .catch(err => console.warn("Docker auto-start trigger:", err))
            .finally(() => {
                invoke('docker_list_images')
                    .then((images: any) => {
                        if (Array.isArray(images)) {
                            // Filter out sha256 and <none>
                            const localImages = images.filter(img => img && !img.includes('sha256:') && !img.includes('<none>'));
                            const uniqueImages = Array.from(new Set([...availableImages, ...localImages]));
                            setAvailableImages(uniqueImages);
                        }
                    })
                    .catch(console.error);
            });
    }, []);

    const handleStart = () => {
        const parsedEnv: Record<string, string> = {};
        envVars.split('\n').forEach(line => {
            const idx = line.indexOf('=');
            if (idx > 0) {
                const key = line.substring(0, idx).trim();
                const value = line.substring(idx + 1).trim();
                if (key) parsedEnv[key] = value;
            }
        });

        let target_type: any = 'DockerImage';
        let baseImage = image;

        if (targetMode === 'snippet') {
            target_type = { CodeSnippet: { language: snippetLang, code: snippetCode } };
            switch (snippetLang) {
                case 'node': baseImage = 'node:26-slim'; break;
                case 'python': baseImage = 'python:3.14-alpine'; break;
                case 'c':
                case 'cpp': baseImage = 'frolvlad/alpine-gxx'; break;
                case 'rust': baseImage = 'rust:alpine'; break;
                case 'go': baseImage = 'golang:alpine'; break;
                case 'java': baseImage = 'openjdk:17-alpine'; break;
                case 'php': baseImage = 'php:8-alpine'; break;
                case 'ruby': baseImage = 'ruby:alpine'; break;
                case 'bash': baseImage = 'alpine:latest'; break;
                default: baseImage = 'alpine:latest'; break;
            }
        } else if (targetMode === 'executable') {
            target_type = { ExecutableFile: { host_path: executablePath } };
            baseImage = 'ubuntu:22.04';
        }

        const config: InspectionConfig = {
            target_type,
            image: baseImage,
            initial_ram_mb: initialRam,
            env_vars: parsedEnv,
            ports: ports.split('\n').map(l => l.trim()).filter(l => l),
            volumes: volumes.split('\n').map(l => l.trim()).filter(l => l),
            network: network.trim() || null,
            cmd: targetMode === 'docker' ? (cmd.trim() || null) : null,
            stress_ramp_rate: 1.5,
            timeout_secs: 300,
        };
        onStart(config);
    };

    return (
        <div className="inspector-control-panel flat-panel">
            <h3 className="panel-title">
                <Settings2 size={16} /> Configuration
            </h3>
            <div style={{ display: 'flex', gap: '8px', marginBottom: '16px' }}>
                <button onClick={() => setTargetMode('docker')} style={{ flex: 1, padding: '8px', background: targetMode === 'docker' ? 'var(--color-accent)' : 'var(--bg-tertiary)', color: targetMode === 'docker' ? '#fff' : 'var(--text-primary)', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>Docker Image</button>
                <button onClick={() => setTargetMode('snippet')} style={{ flex: 1, padding: '8px', background: targetMode === 'snippet' ? 'var(--color-accent)' : 'var(--bg-tertiary)', color: targetMode === 'snippet' ? '#fff' : 'var(--text-primary)', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>Code Snippet</button>
                <button onClick={() => setTargetMode('executable')} style={{ flex: 1, padding: '8px', background: targetMode === 'executable' ? 'var(--color-accent)' : 'var(--bg-tertiary)', color: targetMode === 'executable' ? '#fff' : 'var(--text-primary)', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>Executable File</button>
            </div>

            {targetMode === 'docker' && (
                <div className="control-group">
                    <label>Container Image</label>
                    <input 
                        type="text" 
                        value={image} 
                        onChange={e => setImage(e.target.value)}
                        disabled={isActive}
                        className="inspector-input"
                        placeholder="e.g. redis:alpine"
                        list="docker-images"
                    />
                    <datalist id="docker-images">
                        {availableImages.map((img, idx) => (
                            <option key={idx} value={img} />
                        ))}
                    </datalist>
                </div>
            )}

            {targetMode === 'snippet' && (
                <>
                    <div className="control-group">
                        <label>Language Runtime</label>
                        <select value={snippetLang} onChange={e => setSnippetLang(e.target.value)} disabled={isActive} className="inspector-input">
                            <option value="python">Python 3.14</option>
                            <option value="node">Node.js 26</option>
                            <option value="c">C (GCC)</option>
                            <option value="cpp">C++ (G++)</option>
                            <option value="rust">Rust 1.96</option>
                            <option value="go">Golang 1.26</option>
                            <option value="java">Java 26</option>
                            <option value="php">PHP 8.5</option>
                            <option value="ruby">Ruby 4.0</option>
                            <option value="bash">Bash Script</option>
                        </select>
                    </div>
                    <div className="control-group">
                        <label>Code Snippet</label>
                        <div style={{ height: '300px', border: '1px solid var(--border-primary)', borderRadius: '4px', overflow: 'hidden' }}>
                            <Editor
                                height="100%"
                                language={getMonacoLanguage(snippetLang)}
                                theme="vs-dark"
                                value={snippetCode}
                                onChange={(value) => setSnippetCode(value || '')}
                                options={{
                                    minimap: { enabled: false },
                                    fontSize: 14,
                                    readOnly: isActive,
                                    scrollBeyondLastLine: false,
                                }}
                            />
                        </div>
                    </div>
                </>
            )}

            {targetMode === 'executable' && (
                <div className="control-group">
                    <label>Host Executable Path</label>
                    <div style={{ display: 'flex', gap: '8px' }}>
                        <input 
                            type="text" 
                            value={executablePath} 
                            onChange={e => setExecutablePath(e.target.value)}
                            disabled={isActive}
                            className="inspector-input"
                            placeholder="/path/to/binary"
                            style={{ flex: 1 }}
                        />
                        <button onClick={handleSelectFile} disabled={isActive} style={{ padding: '0 12px', background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-primary)', borderRadius: '4px', cursor: 'pointer' }}>Browse</button>
                    </div>
                </div>
            )}
            <div className="control-group">
                <label>Baseline RAM (MB)</label>
                <input 
                    type="number" 
                    value={initialRam} 
                    onChange={e => setInitialRam(Number(e.target.value))}
                    disabled={isActive}
                    className="inspector-input"
                />
            </div>

            <div 
                className="advanced-settings-toggle"
                onClick={() => setShowAdvanced(!showAdvanced)}
            >
                {showAdvanced ? 'Hide Advanced Settings' : 'Show Advanced Settings'}
            </div>

            <div className={`advanced-settings-container ${showAdvanced ? 'open' : 'closed'}`}>
                    <div className="control-group">
                        <label>Environment Variables (One KEY=VALUE per line)</label>
                        <textarea 
                            value={envVars} 
                            onChange={e => setEnvVars(e.target.value)}
                            disabled={isActive}
                            className="inspector-input"
                            placeholder="NODE_ENV=production&#10;API_KEY=12345"
                            style={{ minHeight: '60px', resize: 'vertical' }}
                        />
                    </div>
                    <div className="control-group">
                        <label>Ports (One port mapping per line)</label>
                        <textarea 
                            value={ports} 
                            onChange={e => setPorts(e.target.value)}
                            disabled={isActive}
                            className="inspector-input"
                            placeholder="8080:80&#10;5432:5432"
                            style={{ minHeight: '40px', resize: 'vertical' }}
                        />
                    </div>
                    <div className="control-group">
                        <label>Volumes (One mount per line)</label>
                        <textarea 
                            value={volumes} 
                            onChange={e => setVolumes(e.target.value)}
                            disabled={isActive}
                            className="inspector-input"
                            placeholder="/host/path:/container/path"
                            style={{ minHeight: '40px', resize: 'vertical' }}
                        />
                    </div>
                    <div className="control-group">
                        <label>Network Mode</label>
                        <input 
                            type="text" 
                            value={network} 
                            onChange={e => setNetwork(e.target.value)}
                            disabled={isActive}
                            className="inspector-input"
                            placeholder="e.g. host, bridge, or my-net"
                        />
                    </div>
                    <div className="control-group">
                        <label>Run Command</label>
                        <input 
                            type="text" 
                            value={cmd} 
                            onChange={e => setCmd(e.target.value)}
                            disabled={isActive}
                            className="inspector-input"
                            placeholder="e.g. npm start"
                        />
                    </div>
                </div>
            
            <div className="control-actions">
                {!isActive ? (
                    <button className="btn-start" onClick={handleStart}>
                        <Play size={16} /> Start Inspection
                    </button>
                ) : (
                    <button className="btn-stop" onClick={onStop}>
                        <Square size={16} /> Stop Inspection
                    </button>
                )}
            </div>
        </div>
    );
}
