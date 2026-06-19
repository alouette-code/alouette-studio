import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { TelemetryData, InspectorState } from '../types';

export function useMemoryInspector() {
    const [history, setHistory] = useState<TelemetryData[]>([]);
    const [state, setState] = useState<InspectorState>({ status: 'Idle' });
    const [isActive, setIsActive] = useState(false);

    useEffect(() => {
        let unlisten: (() => void) | undefined;

        async function setupListener() {
            unlisten = await listen<TelemetryData>('memory-inspector-telemetry', (event) => {
                setHistory(prev => {
                    const newHistory = [...prev, event.payload];
                    // keep last 100 points for chart
                    if (newHistory.length > 100) return newHistory.slice(newHistory.length - 100);
                    return newHistory;
                });
                
                // If we see limits dropping, we are in stress mode
                if (event.payload.memory_limit_mb < (history[0]?.memory_limit_mb || Infinity)) {
                    setState({ status: 'StressTesting' });
                } else {
                    setState({ status: 'BaselineProfiling' });
                }
            });
        }

        if (isActive) {
            setupListener();
        }

        return () => {
            if (unlisten) unlisten();
        };
    }, [isActive, history]);

    const startInspection = async (projectId: string, image: string, initialRam: number) => {
        try {
            setHistory([]);
            setState({ status: 'Isolating' });
            setIsActive(true);
            await invoke('start_memory_inspection', {
                projectId,
                image,
                initialRam
            });
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
        startInspection,
        stopInspection
    };
}
