import { useMemoryInspector } from './hooks/useMemoryInspector';
import { ControlPanel } from './components/ControlPanel';
import { PressureChamberChart } from './components/PressureChamberChart';
import { HeatmapTimeline } from './components/HeatmapTimeline';
import { InsightsPanel } from './components/InsightsPanel';
import { Minus, Square, X } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import brandIcon from '../logo_alouette.png';
import WindowResizer from '../WindowResizer';
import './styles.css';

interface MemoryInspectorProps {
    onClose?: () => void;
}

export function MemoryInspector({ onClose }: MemoryInspectorProps) {
    const { history, state, isActive, startInspection, stopInspection } = useMemoryInspector();
    const appWindow = getCurrentWindow();
    
    const latestData = history.length > 0 ? history[history.length - 1] : null;

    const handleMinimize = async () => {
        try { await appWindow.minimize(); } catch { }
    };
    const handleMaximize = async () => {
        try { await appWindow.toggleMaximize(); } catch { }
    };
    const handleClose = async () => {
        if (onClose) {
            onClose();
        } else {
            try { await appWindow.close(); } catch { }
        }
    };

    return (
        <div className="inspector-container">
            <WindowResizer />
            {/* Titlebar */}
            <div className="postman-window-titlebar" data-tauri-drag-region>
                <div className="titlebar-left" data-tauri-drag-region>
                    <img
                        src={brandIcon}
                        alt="Alouette Logo"
                        className="titlebar-icon"
                        style={{ width: "14px", height: "14px", objectFit: "contain", marginRight: "4px" }}
                    />
                    <span className="titlebar-title">Proto-Memory Inspector</span>
                    <span className="titlebar-subtitle">System & Container Diagnostics</span>
                </div>
                <div className="titlebar-right" data-tauri-drag-region="false">
                    <button
                        className="window-control-btn minimize"
                        onClick={handleMinimize}
                        title="Minimize"
                        data-tauri-drag-region="false"
                    >
                        <Minus size={13} data-tauri-drag-region="false" />
                    </button>
                    <button
                        className="window-control-btn maximize"
                        onClick={handleMaximize}
                        title="Maximize"
                        data-tauri-drag-region="false"
                    >
                        <Square size={10} data-tauri-drag-region="false" />
                    </button>
                    <button
                        className="window-control-btn close"
                        onClick={handleClose}
                        title="Close"
                        data-tauri-drag-region="false"
                    >
                        <X size={14} data-tauri-drag-region="false" />
                    </button>
                </div>
            </div>
            
            {/* Main Body */}
            <div className="inspector-body" style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
                <div className="inspector-left-col">
                    <ControlPanel 
                        isActive={isActive} 
                        onStart={startInspection} 
                        onStop={stopInspection} 
                    />
                    <InsightsPanel 
                        state={state} 
                        latestData={latestData} 
                    />
                </div>
                
                <div className="inspector-right-col" style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
                    <PressureChamberChart data={history} />
                    <HeatmapTimeline data={history} />
                </div>
            </div>
        </div>
    );
}
