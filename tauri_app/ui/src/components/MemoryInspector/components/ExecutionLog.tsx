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
        <div className="flat-panel">
            <h3 className="panel-title">
                <Terminal size={16} /> Execution Log
            </h3>
            
            <div className="execution-timeline">
                {STAGES.map((stage, index) => {
                    if (stage === 'Idle') return null;

                    const isPast = currentIndex > index;
                    const isCurrent = currentIndex === index;
                    const isError = state.status === 'Error' && isCurrent;

                    let statusClass = '';
                    if (isPast) statusClass = 'past';
                    if (isCurrent) statusClass = isError ? 'error active' : 'active';

                    return (
                        <div key={stage} className={`timeline-item ${statusClass}`}>
                            <div className="timeline-dot"></div>
                            <span className="timeline-content">
                                {stage.replace(/([A-Z])/g, ' $1').trim()}
                            </span>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
