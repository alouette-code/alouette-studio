import { TelemetryData, InspectorState } from '../types';
import { Activity, AlertTriangle, CheckCircle } from 'lucide-react';

interface InsightsProps {
    state: InspectorState;
    latestData: TelemetryData | null;
}

export function InsightsPanel({ state, latestData }: InsightsProps) {
    let statusIcon: React.ReactNode = <Activity className="text-blue-400" />;
    let statusText = "Initializing...";

    const driftRateKb = latestData?.drift_rate_kb_per_sec ?? 0;
    const r2Score = latestData?.regression_r2 ?? 0;
    const isStealthyDrift = r2Score >= 0.70 && driftRateKb > 0.005;

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
            } else if (isStealthyDrift) {
                statusText = `MICRO-LEAK DETECTED: Stealthy drift rate +${driftRateKb.toFixed(2)} KB/s (R² = ${(r2Score * 100).toFixed(1)}%)`;
                statusIcon = <AlertTriangle className="text-yellow-500 animate-pulse" />;
            }
            break;
        case 'SmartInspection':
        case 'Finished':
            statusText = isStealthyDrift ? "Inspection Complete - Stealthy Micro-Leak Identified!" : "Inspection Complete.";
            statusIcon = isStealthyDrift ? <AlertTriangle className="text-yellow-500" /> : <CheckCircle className="text-green-500" />;
            break;
        default:
            if (state.status.startsWith('Error')) {
                statusText = state.status;
                statusIcon = <AlertTriangle className="text-red-500" />;
            }
            break;
    }

    return (
        <div className="flat-panel">
            <h3 className="panel-title">Smart Insights</h3>
            
            {state.status.startsWith('Error') ? (
                <div className="alert-box">
                    <div className="alert-icon">{statusIcon}</div>
                    <div className="alert-content">{statusText}</div>
                </div>
            ) : (
                <div className="status-indicator" style={{ display: statusIcon || statusText ? 'flex' : 'none' }}>
                    {statusIcon && <div className="alert-icon" style={{ color: isStealthyDrift ? 'var(--color-warning)' : 'var(--color-info)' }}>{statusIcon}</div>}
                    <span className="status-text">{statusText}</span>
                </div>
            )}

            {latestData && (
                <div className="metrics-grid" style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(100px, 1fr))' }}>
                    <div className="metric-card">
                        <div className="metric-label">Current Usage</div>
                        <div className="metric-value">{latestData.memory_usage_mb.toFixed(1)} MB</div>
                    </div>
                    <div className="metric-card">
                        <div className="metric-label">Current Limit</div>
                        <div className="metric-value">{latestData.memory_limit_mb.toFixed(1)} MB</div>
                    </div>
                    <div className="metric-card">
                        <div className="metric-label">Drift Rate</div>
                        <div className="metric-value" style={{ color: driftRateKb > 0.005 ? 'var(--color-warning)' : 'inherit' }}>
                            {driftRateKb > 0 ? `+${driftRateKb.toFixed(2)} KB/s` : `${driftRateKb.toFixed(2)} KB/s`}
                        </div>
                    </div>
                    <div className="metric-card">
                        <div className="metric-label">Linear Fit (R²)</div>
                        <div className="metric-value">
                            {(r2Score * 100).toFixed(1)}%
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
