import { ProcessActivity } from '../types';
import { Terminal } from 'lucide-react';

interface ActivityLogProps {
    activities?: ProcessActivity[];
}

export function ActivityLog({ activities }: ActivityLogProps) {
    if (!activities) {
        return null;
    }

    // Deduplicate or limit activities if needed. For now, just show them.
    // Reversing to show latest first.
    const displayActivities = [...activities].reverse().slice(0, 50);

    return (
        <div className="flat-panel" style={{ marginTop: '24px' }}>
            <h3 className="panel-title">
                <Terminal size={16} /> System & Process Activity
            </h3>
            
            <div style={{ overflowX: 'auto' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.85rem' }}>
                    <thead>
                        <tr style={{ borderBottom: '1px solid var(--border-primary)', textAlign: 'left', color: 'var(--text-secondary)' }}>
                            <th style={{ padding: '8px' }}>Time</th>
                            <th style={{ padding: '8px' }}>Type</th>
                            <th style={{ padding: '8px' }}>PID/Source</th>
                            <th style={{ padding: '8px' }}>Details</th>
                        </tr>
                    </thead>
                    <tbody>
                        {displayActivities.length === 0 ? (
                            <tr>
                                <td colSpan={4} style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted)' }}>
                                    No activities detected yet.
                                </td>
                            </tr>
                        ) : (
                            displayActivities.map((act, idx) => {
                                const timeStr = new Date(act.timestamp * 1000).toLocaleTimeString();
                                let typeColor = 'var(--color-accent)'; // default blue for generic logs/processes
                                
                                if (act.event_type === 'Memory Syscall') typeColor = '#f59e0b'; // amber
                                else if (act.event_type === 'Network Syscall') typeColor = '#10b981'; // emerald
                                else if (act.event_type === 'File Syscall') typeColor = '#8b5cf6'; // violet
                                else if (act.event_type === 'Syscall') typeColor = '#64748b'; // slate

                                return (
                                    <tr key={idx} style={{ borderBottom: '1px solid var(--border-primary)' }}>
                                        <td style={{ padding: '8px', color: 'var(--text-muted)' }}>{timeStr}</td>
                                        <td style={{ padding: '8px', color: typeColor, fontWeight: 500 }}>{act.event_type}</td>
                                        <td style={{ padding: '8px', fontFamily: 'monospace' }}>{act.pid}</td>
                                        <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--text-secondary)' }}>
                                            {act.details}
                                        </td>
                                    </tr>
                                );
                            })
                        )}
                    </tbody>
                </table>
            </div>
        </div>
    );
}
