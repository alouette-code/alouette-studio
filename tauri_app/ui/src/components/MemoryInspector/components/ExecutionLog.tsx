import { InspectorState } from '../types';
import { Terminal, CheckCircle2, CircleDashed, Circle } from 'lucide-react';

interface ExecutionLogProps {
    state: InspectorState;
}

const STAGES = [
    'Idle',
    'PreFlightChecks',
    'Isolating',
    'BaselineProfiling',
    'StressTesting',
    'SmartInspection',
    'GeneratingReport',
    'Finished'
];

export function ExecutionLog({ state }: ExecutionLogProps) {
    const currentIndex = STAGES.indexOf(state.status);

    return (
        <div className="execution-log-panel" style={{ marginTop: '20px', background: 'var(--bg-secondary)', borderRadius: '6px', border: '1px solid var(--border-primary)', padding: '16px' }}>
            <h3 style={{ margin: '0 0 16px 0', display: 'flex', alignItems: 'center', gap: '8px', fontSize: '13px', textTransform: 'uppercase', color: 'var(--text-secondary)' }}>
                <Terminal size={14} /> Execution Log
            </h3>
            
            <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                {STAGES.map((stage, index) => {
                    // Skip idle
                    if (stage === 'Idle') return null;

                    const isPast = currentIndex > index;
                    const isCurrent = currentIndex === index;
                    const isFuture = currentIndex < index;
                    const isError = state.status === 'Error' && isCurrent;

                    let color = 'var(--text-muted)';
                    let icon = <Circle size={14} />;
                    
                    if (isPast) {
                        color = 'var(--color-success)';
                        icon = <CheckCircle2 size={14} />;
                    } else if (isCurrent) {
                        color = 'var(--color-info)';
                        icon = <CircleDashed size={14} className="spin-animation" />;
                        if (isError) {
                            color = 'var(--color-danger)';
                        }
                    }

                    return (
                        <div key={stage} style={{ display: 'flex', alignItems: 'center', gap: '10px', color, opacity: isFuture ? 0.5 : 1 }}>
                            {icon}
                            <span style={{ fontSize: '12px', fontWeight: isCurrent ? 'bold' : 'normal' }}>
                                {stage.replace(/([A-Z])/g, ' $1').trim()}
                            </span>
                        </div>
                    );
                })}

                {state.status === 'Error' && (
                    <div style={{ marginTop: '10px', padding: '10px', background: 'rgba(255,50,50,0.1)', borderLeft: '3px solid var(--color-danger)', color: 'var(--color-danger)', fontSize: '11px', fontFamily: 'monospace', wordBreak: 'break-all' }}>
                        ERROR: {state.error}
                    </div>
                )}
            </div>
        </div>
    );
}
