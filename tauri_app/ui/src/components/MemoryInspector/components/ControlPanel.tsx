import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Play, Square, Settings2 } from 'lucide-react';
import { InspectionConfig } from '../types';

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
    const [availableImages, setAvailableImages] = useState<string[]>([
        'redis:alpine', 'nginx:latest', 'node:18-alpine', 'python:3.9-slim', 'postgres:15-alpine'
    ]);

    useEffect(() => {
        // Fetch all local docker images
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

        const config: InspectionConfig = {
            image,
            initial_ram_mb: initialRam,
            env_vars: parsedEnv,
            ports: ports.split('\n').map(l => l.trim()).filter(l => l),
            volumes: volumes.split('\n').map(l => l.trim()).filter(l => l),
            network: network.trim() || null,
            cmd: cmd.trim() || null,
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
