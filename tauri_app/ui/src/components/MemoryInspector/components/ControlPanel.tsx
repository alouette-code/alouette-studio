import { useState } from 'react';
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
    const [cmd, setCmd] = useState('');
    const [showAdvanced, setShowAdvanced] = useState(false);

    const handleStart = () => {
        const config: InspectionConfig = {
            image,
            initial_ram_mb: initialRam,
            env_vars: envVars.split(',').map(e => e.trim()).filter(e => e.length > 0),
            cmd: cmd.trim() || null,
            stress_ramp_rate: 1.5,
            timeout_secs: 300,
        };
        onStart(config);
    };

    return (
        <div className="inspector-control-panel">
            <div className="control-group">
                <label>Container Image</label>
                <input 
                    type="text" 
                    value={image} 
                    onChange={e => setImage(e.target.value)}
                    disabled={isActive}
                    className="inspector-input"
                    placeholder="e.g. redis:alpine"
                />
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

            {showAdvanced && (
                <div className="advanced-settings" style={{ marginTop: '10px', padding: '10px', background: 'var(--bg-tertiary)', borderRadius: '4px' }}>
                    <div className="control-group">
                        <label>Environment Variables (comma-separated)</label>
                        <input 
                            type="text" 
                            value={envVars} 
                            onChange={e => setEnvVars(e.target.value)}
                            disabled={isActive}
                            className="inspector-input"
                            placeholder="KEY=VALUE, NODE_ENV=prod"
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
            )}
            
            <div className="control-actions" style={{ marginTop: '16px' }}>
                {!isActive ? (
                    <button className="btn-start" onClick={handleStart}>
                        <Play size={16} /> Start Inspection
                    </button>
                ) : (
                    <button className="btn-stop" onClick={onStop}>
                        <Square size={16} /> Stop Testing
                    </button>
                )}
                
                <button 
                    className={`btn-icon ${showAdvanced ? 'active' : ''}`} 
                    title="Advanced Settings"
                    onClick={() => setShowAdvanced(!showAdvanced)}
                >
                    <Settings2 size={16} />
                </button>
            </div>
        </div>
    );
}
