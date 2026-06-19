import { useState } from 'react';
import { Play, Square, Settings2 } from 'lucide-react';

interface ControlPanelProps {
    isActive: boolean;
    onStart: (projectId: string, image: string, initialRam: number) => void;
    onStop: () => void;
}

export function ControlPanel({ isActive, onStart, onStop }: ControlPanelProps) {
    const [image, setImage] = useState('alpine:latest'); // Mock image for demo
    const [initialRam, setInitialRam] = useState(512);

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
            
            <div className="control-actions">
                {!isActive ? (
                    <button className="btn-start" onClick={() => onStart("demo-project", image, initialRam)}>
                        <Play size={16} /> Start Inspection
                    </button>
                ) : (
                    <button className="btn-stop" onClick={onStop}>
                        <Square size={16} /> Stop Testing
                    </button>
                )}
                
                <button className="btn-icon" title="Mock Injection Settings">
                    <Settings2 size={16} />
                </button>
            </div>
        </div>
    );
}
