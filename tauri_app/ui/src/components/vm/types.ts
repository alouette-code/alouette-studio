export interface AdvancedConfig {
  audio_enabled: boolean;
  tpm_enabled: boolean;
  graceful_shutdown_timeout: number;
  host_port_forwards: [number, number][];
  additional_disks: string[];
}

export interface VmConfig {
  id: string;
  name: string;
  os_type: string | null;
  arch?: "x86_64" | "aarch64" | string | null;
  cpu_cores: number;
  ram_size_mb: number;
  vm_dir: string;
  iso_path: string | null;
  disk_path: string | null;
  disk_size_gb: number | null;
  network_mode: string;
  firmware: "bios" | "uefi" | string | null;
  advanced: AdvancedConfig;
}

export interface VmRuntimeInfo {
  config: VmConfig;
  status: "running" | "stopped" | string;
  ws_port: number | null;
  vnc_display: number | null;
}

export interface VmItem {
  id: string;
  name: string;
  status: "running" | "stopped" | string;
  ws_port: number | null;
  vnc_display: number | null;
  config: VmConfig;
}

export interface OsTemplate {
  id: string;
  label: string;
  desc: string;
  conf: {
    cpu: number;
    ramMb: number;
    diskGb: number;
    fw: "bios" | "uefi";
    arch?: "x86_64" | "aarch64";
    tpm?: boolean;
  };
}

export interface OsFamily {
  label: string;
  versions: OsTemplate[];
}

export interface VmNotice {
  type: "success" | "error" | "info";
  message: string;
}
