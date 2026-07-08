export interface ProcessActivity {
    timestamp: number;
    event_type: string;
    pid: string;
    details: string;
}

export interface TelemetryData {
    timestamp: number;
    memory_usage_mb: number;
    memory_limit_mb: number;
    thread_count: number;
    gc_events_detected: number;
    crash_imminent: boolean;
    activities?: ProcessActivity[];
}

export type Diagnosis = 'CacheEviction' | 'StubbornLeak' | 'Unknown';

export interface InspectorState {
    status: 'Idle' | 'PreFlightChecks' | 'Isolating' | 'BaselineProfiling' | 'StressTesting' | 'SmartInspection' | 'GeneratingReport' | 'Finished' | 'Error';
    error?: string;
}

export type TargetType = 
    | 'DockerImage'
    | { CodeSnippet: { language: string; code: string } }
    | { ExecutableFile: { host_path: string } };

export interface InspectionConfig {
    target_type?: TargetType;
    image: string;
    initial_ram_mb: number;
    env_vars: Record<string, string>;
    ports: string[];
    volumes: string[];
    network: string | null;
    cmd: string | null;
    stress_ramp_rate: number;
    timeout_secs: number;
}

export interface TaskRecord {
    task_id: string;
    config: InspectionConfig;
    start_time: number;
    end_time: number | null;
    status: string;
    final_diagnosis: Diagnosis | null;
}
