import { useEffect } from 'react';
import { TaskRecord } from '../types';
import { Database, Clock, Activity, CheckCircle2, XCircle } from 'lucide-react';

interface TaskHistoryProps {
    tasks: TaskRecord[];
    onRefresh: () => void;
}

export function TaskHistory({ tasks, onRefresh }: TaskHistoryProps) {
    useEffect(() => {
        onRefresh();
    }, []);

    const formatTime = (ts: number) => {
        return new Date(ts * 1000).toLocaleString();
    };

    return (
        <div className="task-history-panel" style={{ marginTop: '20px', background: 'var(--bg-secondary)', borderRadius: '6px', border: '1px solid var(--border-primary)', padding: '16px' }}>
            <h3 style={{ margin: '0 0 16px 0', display: 'flex', alignItems: 'center', gap: '8px', fontSize: '13px', textTransform: 'uppercase', color: 'var(--text-secondary)' }}>
                <Database size={14} /> Task History ({tasks.length})
            </h3>
            
            {tasks.length === 0 ? (
                <div style={{ textAlign: 'center', color: 'var(--text-muted)', fontStyle: 'italic', padding: '20px' }}>
                    No inspection tasks found.
                </div>
            ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', maxHeight: '300px', overflowY: 'auto' }} className="custom-scrollbar">
                    {tasks.map(task => (
                        <div key={task.task_id} style={{ background: 'var(--bg-tertiary)', padding: '10px', borderRadius: '4px', border: '1px solid var(--border-primary)', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                <span style={{ fontFamily: 'monospace', color: 'var(--color-info)', fontSize: '11px' }}>{task.task_id}</span>
                                {task.status === 'Running' ? (
                                    <span style={{ color: 'var(--color-warning)', display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px' }}><Activity size={12} /> Running</span>
                                ) : task.status === 'Finished' ? (
                                    <span style={{ color: 'var(--color-success)', display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px' }}><CheckCircle2 size={12} /> Finished</span>
                                ) : (
                                    <span style={{ color: 'var(--color-danger)', display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px' }}><XCircle size={12} /> {task.status}</span>
                                )}
                            </div>
                            <div style={{ fontSize: '12px', color: 'var(--text-primary)' }}>
                                <strong>Image:</strong> {task.config.image} | <strong>RAM:</strong> {task.config.initial_ram_mb}MB
                            </div>
                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: 'var(--text-muted)' }}>
                                <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}><Clock size={10} /> {formatTime(task.start_time)}</span>
                                {task.final_diagnosis && (
                                    <span style={{ color: 'var(--color-warning)' }}>Diagnosis: {task.final_diagnosis}</span>
                                )}
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
