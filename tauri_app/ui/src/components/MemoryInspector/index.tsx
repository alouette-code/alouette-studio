import { useMemoryInspector } from './hooks/useMemoryInspector';
import { ControlPanel } from './components/ControlPanel';
import { PressureChamberChart } from './components/PressureChamberChart';
import { HeatmapTimeline } from './components/HeatmapTimeline';
import { InsightsPanel } from './components/InsightsPanel';
import { TaskHistory } from './components/TaskHistory';
import { ExecutionLog } from './components/ExecutionLog';
import { getCurrentWindow } from '@tauri-apps/api/window';
import brandIcon from '../logo_alouette.png';
import WindowResizer from '../WindowResizer';
import { WindowControls } from '../WindowControls';
import './styles.css';

interface MemoryInspectorProps {
    onClose?: () => void;
}

export function MemoryInspector({ onClose }: MemoryInspectorProps) {
    const { history, state, isActive, tasks, startInspection, stopInspection, fetchTaskHistory } = useMemoryInspector();
    const appWindow = getCurrentWindow();
    
    const latestData = history.length > 0 ? history[history.length - 1] : null;

    return (
        <div className="inspector-container">
            <WindowResizer />
            {/* Titlebar */}
            <div className="pingzero-window-titlebar" data-tauri-drag-region>
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
                    <WindowControls />
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
                    <ExecutionLog state={state} />
                    <InsightsPanel 
                        state={state} 
                        latestData={latestData} 
                    />
                    <TaskHistory 
                        tasks={tasks}
                        onRefresh={fetchTaskHistory}
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
