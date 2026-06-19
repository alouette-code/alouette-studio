export interface TelemetryData {
    timestamp: number;
    memory_usage_mb: number;
    memory_limit_mb: number;
    gc_events_detected: number;
    crash_imminent: boolean;
}

export type Diagnosis = 'CacheEviction' | 'StubbornLeak' | 'Unknown';

export interface InspectorState {
    status: 'Idle' | 'Isolating' | 'BaselineProfiling' | 'StressTesting' | 'SmartInspection' | 'Finished' | 'Error';
    error?: string;
}
