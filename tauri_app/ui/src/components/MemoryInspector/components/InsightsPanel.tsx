import { TelemetryData, InspectorState } from '../types';
import { Activity, AlertTriangle, CheckCircle } from 'lucide-react';

interface InsightsProps {
    state: InspectorState;
    latestData: TelemetryData | null;
}

export function InsightsPanel({ state, latestData }: InsightsProps) {
    let statusIcon: React.ReactNode = <Activity className="text-blue-400" />;
    let statusText = "Initializing...";

    switch (state.status) {
        case 'Idle':
            statusText = "Ready to inspect.";
            statusIcon = null;
            break;
        case 'Isolating':
            statusText = "Creating Sandbox Environment...";
            break;
        case 'BaselineProfiling':
            statusText = "Gathering Baseline...";
            break;
        case 'StressTesting':
            statusText = "Pressure Chamber Active - Ramp Down initiated.";
            statusIcon = <AlertTriangle className="text-orange-500 animate-pulse" />;
            if (latestData?.crash_imminent) {
                statusText = "CRITICAL: Crash Imminent. Exponential growth detected.";
                statusIcon = <AlertTriangle className="text-red-500 animate-pulse" />;
            }
            break;
        case 'SmartInspection':
        case 'Finished':
            statusText = "Inspection Complete.";
            statusIcon = <CheckCircle className="text-green-500" />;
            break;
        case 'Error':
            statusText = `Error: ${state.error}`;
            statusIcon = <AlertTriangle className="text-red-500" />;
            break;
    }

    return (
        <div className="flat-panel">
            <h3 className="panel-title">Smart Insights</h3>
            
            {state.status === 'Error' ? (
                <div className="alert-box">
                    <div className="alert-icon">{statusIcon}</div>
                    <div className="alert-content">{statusText}</div>
                </div>
            ) : (
                <div className="status-indicator" style={{ display: statusIcon || statusText ? 'flex' : 'none' }}>
                    {statusIcon && <div className="alert-icon" style={{ color: 'var(--color-info)' }}>{statusIcon}</div>}
                    <span className="status-text">{statusText}</span>
                </div>
            )}

            {latestData && (
                <div className="metrics-grid">
                    <div className="metric-card">
                        <div className="metric-label">Current Usage</div>
                        <div className="metric-value">{latestData.memory_usage_mb.toFixed(1)} MB</div>
                    </div>
                    <div className="metric-card">
                        <div className="metric-label">Current Limit</div>
                        <div className="metric-value">{latestData.memory_limit_mb.toFixed(1)} MB</div>
                    </div>
                    <div className="metric-card">
                        <div className="metric-label">GC Events</div>
                        <div className="metric-value">{latestData.gc_events_detected}</div>
                    </div>
                </div>
            )}
        </div>
    );
}
