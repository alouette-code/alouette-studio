import { TelemetryData } from '../types';

interface HeatmapProps {
    data: TelemetryData[];
}

export function HeatmapTimeline({ data }: HeatmapProps) {
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
