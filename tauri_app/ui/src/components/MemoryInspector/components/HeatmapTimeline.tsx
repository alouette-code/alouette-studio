import { TelemetryData, TaskRecord } from '../types';

interface HeatmapProps {
    data: TelemetryData[];
    currentTask?: TaskRecord;
}

export function HeatmapTimeline({ data, currentTask }: HeatmapProps) {
    // If there is a diagnosis, show the detailed culprit report instead of the heatmap
    if (currentTask?.final_diagnosis) {
        return (
            <div className="inspector-heatmap-container" style={{ display: 'flex', flexDirection: 'column' }}>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
                    <h3 className="chart-title" style={{ margin: 0, color: 'var(--color-warning)' }}>
                        Auto-Diagnosis Report: {currentTask.final_diagnosis}
                    </h3>
                </div>
                
                <div style={{ flex: 1, overflowY: 'auto', background: 'var(--bg-tertiary)', borderRadius: '6px', border: '1px solid var(--border-primary)', padding: '12px' }} className="custom-scrollbar">
                    <h4 style={{ margin: '0 0 10px 0', fontSize: '13px', color: 'var(--text-secondary)' }}>
                        Top Culprits (Commands / Functions causing the leak)
                    </h4>
                    
                    {!currentTask.culprit_summary || currentTask.culprit_summary.length === 0 ? (
                        <div style={{ color: 'var(--text-muted)', fontStyle: 'italic', fontSize: '12px' }}>
                            No specific culprits found in logs.
                        </div>
                    ) : (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                            {currentTask.culprit_summary.map((culprit, idx) => (
                                <div key={idx} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--bg-secondary)', padding: '8px 12px', borderRadius: '4px', borderLeft: `3px solid ${idx === 0 ? 'var(--color-danger)' : idx === 1 ? 'var(--color-warning)' : 'var(--color-info)'}` }}>
                                    <span style={{ fontFamily: 'monospace', fontSize: '12px', color: 'var(--text-primary)', wordBreak: 'break-all' }}>
                                        {culprit.name}
                                    </span>
                                    <span style={{ fontSize: '11px', fontWeight: 'bold', color: 'var(--text-muted)', background: 'var(--bg-primary)', padding: '4px 8px', borderRadius: '12px', whiteSpace: 'nowrap', marginLeft: '12px' }}>
                                        {culprit.count}x calls
                                    </span>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        );
    }

    // Generate heatmap blocks. Color intensity based on how close usage is to limit
    const blocks = data.map((d, i) => {
        const ratio = d.memory_limit_mb > 0 ? (d.memory_usage_mb / d.memory_limit_mb) : 0;
        let colorClass = "bg-green-500/20";
        if (ratio > 0.95) colorClass = "bg-red-500";
        else if (ratio > 0.85) colorClass = "bg-orange-500";
        else if (ratio > 0.7) colorClass = "bg-yellow-500";

        return (
            <div 
                key={i} 
                className={`heatmap-block ${colorClass}`}
                title={`Usage: ${d.memory_usage_mb.toFixed(1)}MB / Limit: ${d.memory_limit_mb.toFixed(1)}MB`}
            />
        );
    });

    return (
        <div className="inspector-heatmap-container">
            <h3 className="chart-title">Leak Heatmap Timeline</h3>
            <div className="heatmap-track">
                {blocks.length > 0 ? blocks : <div className="heatmap-empty">Awaiting data...</div>}
            </div>
            <div className="heatmap-legend">
                <span className="legend-item"><span className="legend-color bg-green-500/20"></span> Stable</span>
                <span className="legend-item"><span className="legend-color bg-yellow-500"></span> Warning</span>
                <span className="legend-item"><span className="legend-color bg-red-500"></span> Critical / Denied Allocation</span>
            </div>
        </div>
    );
}
