import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { TelemetryData, InspectorState, InspectionConfig, TaskRecord } from '../types';

export function useMemoryInspector() {
    const [history, setHistory] = useState<TelemetryData[]>([]);
    const [state, setState] = useState<InspectorState>({ status: 'Idle' });
    const [isActive, setIsActive] = useState(false);
    const [tasks, setTasks] = useState<TaskRecord[]>([]);
    const [currentTaskId, setCurrentTaskId] = useState<string | null>(null);

    useEffect(() => {
        let unlisten: (() => void) | undefined;

        async function setupListener() {
            unlisten = await listen<TelemetryData & { status: string }>('memory-inspector-telemetry', (event) => {
                setHistory(prev => {
                    const newHistory = [...prev, event.payload];
                    // keep last 100 points for chart
                    if (newHistory.length > 100) return newHistory.slice(newHistory.length - 100);
                    return newHistory;
                });
                
                // Extract status from backend payload instead of guessing
                setState(prev => {
                    if (prev.status !== event.payload.status) {
                        return { status: event.payload.status as any };
                    }
                    return prev;
                });
            });
        }

        if (isActive) {
            setupListener();
        }

        return () => {
            if (unlisten) unlisten();
        };
    }, [isActive]);


    const fetchTaskHistory = useCallback(async () => {
        try {
            const history = await invoke<TaskRecord[]>('get_task_history');
            setTasks(history);
        } catch (e) {
            console.error("Failed to fetch task history:", e);
        }
    }, []);

    useEffect(() => {
        if (state.status === 'Finished' || state.status === 'Error' || state.status.startsWith('Error') || (state as any).error) {
            setIsActive(false);
            fetchTaskHistory(); // refresh history to get final diagnosis
        }
    }, [state.status, fetchTaskHistory]);

    const startInspection = async (config: InspectionConfig) => {
        try {
            setHistory([]);
            setState({ status: 'PreFlightChecks' });
            setIsActive(true);
            const taskId = await invoke<string>('start_memory_inspection', { config });
            setCurrentTaskId(taskId);
            fetchTaskHistory();
        } catch (e: any) {
            setState({ status: 'Error', error: e.toString() });
            setIsActive(false);
        }
    };

    const stopInspection = async () => {
        try {
            await invoke('stop_memory_inspection');
            setState({ status: 'Finished' });
            setIsActive(false);
        } catch (e: any) {
            console.error(e);
        }
    };

    return {
        history,
        state,
        isActive,
        tasks,
        currentTaskId,
        startInspection,
        stopInspection,
        fetchTaskHistory
    };
}
