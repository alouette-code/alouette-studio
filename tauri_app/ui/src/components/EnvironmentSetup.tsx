import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Project } from "../types";
import {
  Eye,
  Lock,
  TrendingUpDown,
  ArrowDown,
  ArrowUp,
  Plus,
  Trash2,
  Shield,
  ChartNetwork,
  Server,
  Cpu,
  Dna,
} from "lucide-react";

export interface SimulatedEnvVar {
  id: string;
  key: string;
  value: string;
  visibility: "exposed" | "hidden";
  scope: "inbound" | "outbound" | "both";
  enabled: boolean;
}

interface EnvSimulationConfig {
  project_id: string;
  firewall_enabled: boolean;
  firewall_rules: string;
  weak_network_enabled: boolean;
  latency_ms: number;
  jitter_ms: number;
  loss_rate: number;
  bandwidth_kbps: number;
  unstable_server_enabled: boolean;
  unstable_server_drop_rate: number;
  unstable_server_periodic_crash_secs: number;
  unstable_server_error_rate: number;
  unstable_server_error_codes: string;
  cpu_limit_enabled: boolean;
  cpu_limit_percent: number;
  ram_limit_enabled: boolean;
  ram_limit_mb: number;
  env_injection_enabled: boolean;
  custom_envs: SimulatedEnvVar[];
}

interface EnvironmentSetupProps {
  activeProject: Project | null;
}

export default function EnvironmentSetup({ activeProject }: EnvironmentSetupProps) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjId, setSelectedProjId] = useState<string>("");
  const [config, setConfig] = useState<EnvSimulationConfig | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">("idle");
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load project list
  useEffect(() => {
    (async () => {
      try {
        const list = await invoke<Project[]>("get_projects");
        setProjects(list);
        if (activeProject) {
          setSelectedProjId(activeProject.id);
        } else if (list.length > 0) {
          setSelectedProjId(list[0].id);
        }
      } catch (e) {
        console.error("Failed to fetch projects list:", e);
      }
    })();
  }, [activeProject]);

  // Load simulation configuration when selected project changes
  useEffect(() => {
    if (!selectedProjId) return;

    (async () => {
      setIsLoading(true);
      setSaveStatus("idle");
      try {
        const configsMap = await invoke<{ [id: string]: EnvSimulationConfig }>("load_env_simulation_configs");
        const rawCfg = configsMap[selectedProjId];
        
        const formatted: EnvSimulationConfig = {
          project_id: selectedProjId,
          firewall_enabled: rawCfg?.firewall_enabled ?? false,
          firewall_rules: rawCfg?.firewall_rules ?? "",
          weak_network_enabled: rawCfg?.weak_network_enabled ?? false,
          latency_ms: rawCfg?.latency_ms ?? 0,
          jitter_ms: rawCfg?.jitter_ms ?? 0,
          loss_rate: rawCfg?.loss_rate ?? 0.0,
          bandwidth_kbps: rawCfg?.bandwidth_kbps ?? 0,
          unstable_server_enabled: rawCfg?.unstable_server_enabled ?? false,
          unstable_server_drop_rate: rawCfg?.unstable_server_drop_rate ?? 0.0,
          unstable_server_periodic_crash_secs: rawCfg?.unstable_server_periodic_crash_secs ?? 0,
          unstable_server_error_rate: rawCfg?.unstable_server_error_rate ?? 0.0,
          unstable_server_error_codes: rawCfg?.unstable_server_error_codes ?? "500,502,503",
          cpu_limit_enabled: rawCfg?.cpu_limit_enabled ?? false,
          cpu_limit_percent: rawCfg?.cpu_limit_percent ?? 80,
          ram_limit_enabled: rawCfg?.ram_limit_enabled ?? false,
          ram_limit_mb: rawCfg?.ram_limit_mb ?? 2000,
          env_injection_enabled: rawCfg?.env_injection_enabled ?? false,
          custom_envs: rawCfg?.custom_envs ?? [],
        };
        setConfig(formatted);
      } catch (e) {
        console.error("Failed to load environment simulation configs:", e);
      } finally {
        setIsLoading(false);
      }
    })();
  }, [selectedProjId]);

  // Debounced auto-save
  const triggerSave = (updated: EnvSimulationConfig) => {
    setIsSaving(true);
    setSaveStatus("idle");
    if (saveTimerRef.current) {
      clearTimeout(saveTimerRef.current);
    }
    saveTimerRef.current = setTimeout(async () => {
      try {
        await invoke("save_env_simulation_config", { config: updated });
        setSaveStatus("success");
      } catch (e) {
        console.error("Failed to auto-save environment settings:", e);
        setSaveStatus("error");
      } finally {
        setIsSaving(false);
      }
    }, 800);
  };

  const updateField = <K extends keyof EnvSimulationConfig>(key: K, value: EnvSimulationConfig[K]) => {
    if (!config) return;
    let sanitizedVal = value;
    if (typeof value === "number") {
      if (isNaN(value)) {
        sanitizedVal = 0 as any;
      } else if (key === "loss_rate" || key === "unstable_server_drop_rate" || key === "unstable_server_error_rate") {
        sanitizedVal = Math.min(100, Math.max(0, value)) as any;
      } else if (key === "cpu_limit_percent") {
        sanitizedVal = Math.min(100, Math.max(1, value)) as any;
      } else {
        sanitizedVal = Math.max(0, value) as any;
      }
    }
    const updated = { ...config, [key]: sanitizedVal };
    setConfig(updated);
    triggerSave(updated);
  };

  const addCustomEnv = (presetKey?: string, presetVal?: string, presetVis?: "exposed" | "hidden", presetScope?: "inbound" | "outbound" | "both") => {
    if (!config) return;
    const newEnv: SimulatedEnvVar = {
      id: "env_" + Date.now() + "_" + Math.floor(Math.random() * 1000),
      key: presetKey ?? "SIM_GATEWAY_MARKER",
      value: presetVal ?? "trace_id_" + Math.floor(Math.random() * 10000),
      visibility: presetVis ?? "hidden",
      scope: presetScope ?? "both",
      enabled: true,
    };
    const updated = {
      ...config,
      env_injection_enabled: true,
      custom_envs: [...(config.custom_envs || []), newEnv],
    };
    setConfig(updated);
    triggerSave(updated);
  };

  const updateCustomEnv = (id: string, field: keyof SimulatedEnvVar, val: any) => {
    if (!config) return;
    const updatedEnvs = (config.custom_envs || []).map((e) => {
      if (e.id === id) {
        return { ...e, [field]: val };
      }
      return e;
    });
    const updated = {
      ...config,
      env_injection_enabled: true,
      custom_envs: updatedEnvs,
    };
    setConfig(updated);
    triggerSave(updated);
  };

  const deleteCustomEnv = (id: string) => {
    if (!config) return;
    const updatedEnvs = (config.custom_envs || []).filter((e) => e.id !== id);
    const updated = { ...config, custom_envs: updatedEnvs };
    setConfig(updated);
    triggerSave(updated);
  };

  const applyFirewallPreset = (preset: string) => {
    if (!config) return;
    let rules = config.firewall_rules;
    if (preset === "local_only") {
      rules = "*.com, *.org, *.net, *.edu, *.gov, *.io, *.co, *.info, *.me, *.dev, *.ai, github.com, google.com";
    } else if (preset === "block_social") {
      rules = "facebook.com, *.facebook.com, twitter.com, *.twitter.com, x.com, *.x.com, instagram.com, *.instagram.com, tiktok.com, *.tiktok.com, youtube.com, *.youtube.com";
    } else if (preset === "block_google") {
      rules = "google.com, *.google.com, googleapis.com, *.googleapis.com, gstatic.com, *.gstatic.com";
    } else if (preset === "clear") {
      rules = "";
    }
    updateField("firewall_rules", rules);
  };

  if (isLoading) {
    return (
      <div style={{ display: "flex", justifyContent: "center", alignItems: "center", height: "100%", backgroundColor: "var(--bg-primary)" }}>
        <p style={{ color: "var(--text-secondary)", fontSize: "11px", fontFamily: "var(--font-mono)" }}>Loading configuration...</p>
      </div>
    );
  }

  // Monochrome styling tokens
  const inputStyle: React.CSSProperties = {
    backgroundColor: "var(--bg-secondary, #181818)",
    border: "1px solid var(--border-primary, #2a2a2a)",
    color: "var(--text-primary, #e0e0e0)",
    padding: "5px 8px",
    fontSize: "11px",
    fontFamily: "var(--font-mono)",
    borderRadius: "3px",
    outline: "none",
    width: "100%",
  };

  const panelStyle: React.CSSProperties = {
    padding: "16px",
    overflowY: "auto",
    height: "100%",
    backgroundColor: "var(--bg-primary, #0f0f0f)",
    color: "var(--text-primary, #e0e0e0)",
    fontFamily: "var(--font-mono, monospace)",
    fontSize: "12px",
  };

  const headerStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    borderBottom: "1px solid var(--border-primary, #2a2a2a)",
    paddingBottom: "12px",
    marginBottom: "16px",
  };

  const cardStyle: React.CSSProperties = {
    border: "1px solid var(--border-primary, #2a2a2a)",
    borderRadius: "4px",
    padding: "12px",
    backgroundColor: "var(--bg-secondary, #141414)",
    display: "flex",
    flexDirection: "column",
    gap: "10px",
  };

  const buttonPresetStyle: React.CSSProperties = {
    fontSize: "9px",
    padding: "3px 7px",
    borderRadius: "3px",
    border: "1px solid var(--border-primary, #2a2a2a)",
    backgroundColor: "rgba(255, 255, 255, 0.04)",
    color: "var(--text-primary, #e0e0e0)",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    gap: "4px",
  };

  return (
    <div style={panelStyle}>
      {/* Header controls */}
      <div style={headerStyle}>
        <div>
          <h2 style={{ fontSize: "13px", fontWeight: 600, margin: "0 0 2px", textTransform: "uppercase", letterSpacing: "0.5px", color: "var(--text-primary)" }}>
            Environment Simulation Setup
          </h2>
          <p style={{ fontSize: "10px", color: "var(--text-secondary)", margin: 0 }}>
            Configuring custom sandboxed conditions & environment injection for the selected project.
          </p>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
          {/* Project Switcher */}
          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
            <span style={{ fontSize: "11px", color: "var(--text-secondary)" }}>Project:</span>
            <select
              value={selectedProjId}
              onChange={(e) => setSelectedProjId(e.target.value)}
              style={{
                backgroundColor: "var(--bg-secondary, #181818)",
                border: "1px solid var(--border-primary, #2a2a2a)",
                color: "var(--text-primary)",
                padding: "3px 8px",
                fontSize: "11px",
                fontWeight: "normal",
                outline: "none",
                borderRadius: "3px",
                fontFamily: "var(--font-mono)",
              }}
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </div>

          {/* Save Status */}
          <div style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
            {isSaving ? "Saving..." : saveStatus === "success" ? "Saved" : saveStatus === "error" ? "Error saving" : "Auto-save"}
          </div>
        </div>
      </div>

      {config ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
            {/* FIREWALL BLOCK */}
            <div style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-primary)", paddingBottom: "6px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <Shield size={14} style={{ color: "var(--text-primary)" }} />
                  <span style={{ fontWeight: 600 }}>Firewall Rules</span>
                </div>
                <input
                  type="checkbox"
                  checked={config.firewall_enabled}
                  onChange={(e) => updateField("firewall_enabled", e.target.checked)}
                  style={{ cursor: "pointer" }}
                />
              </div>

              <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
                <button
                  type="button"
                  onClick={() => applyFirewallPreset("local_only")}
                  style={buttonPresetStyle}
                >
                  Local Only
                </button>
                <button
                  type="button"
                  onClick={() => applyFirewallPreset("block_social")}
                  style={buttonPresetStyle}
                >
                  Block Socials
                </button>
                <button
                  type="button"
                  onClick={() => applyFirewallPreset("block_google")}
                  style={buttonPresetStyle}
                >
                  Block Google
                </button>
                <button
                  type="button"
                  onClick={() => applyFirewallPreset("clear")}
                  style={{ ...buttonPresetStyle, backgroundColor: "transparent", color: "var(--text-secondary)" }}
                >
                  Clear
                </button>
              </div>

              <div>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>
                  Blocked domains/IPs (comma separated):
                </label>
                <textarea
                  rows={3}
                  disabled={!config.firewall_enabled}
                  value={config.firewall_rules}
                  onChange={(e) => updateField("firewall_rules", e.target.value)}
                  placeholder="e.g. *.google.com, github.com, 1.1.1.1"
                  style={{ ...inputStyle, resize: "vertical" }}
                />
              </div>
            </div>

            {/* NETWORK SIMULATION BLOCK */}
            <div style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-primary)", paddingBottom: "6px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <ChartNetwork size={14} style={{ color: "var(--text-primary)" }} />
                  <span style={{ fontWeight: 600 }}>Network Simulation</span>
                </div>
                <input
                  type="checkbox"
                  checked={config.weak_network_enabled}
                  onChange={(e) => updateField("weak_network_enabled", e.target.checked)}
                  style={{ cursor: "pointer" }}
                />
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "10px" }}>
                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Latency (ms):</label>
                  <input
                    type="number"
                    disabled={!config.weak_network_enabled}
                    value={config.latency_ms}
                    onChange={(e) => updateField("latency_ms", parseInt(e.target.value) || 0)}
                    style={inputStyle}
                    min="0"
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Jitter (ms):</label>
                  <input
                    type="number"
                    disabled={!config.weak_network_enabled}
                    value={config.jitter_ms}
                    onChange={(e) => updateField("jitter_ms", parseInt(e.target.value) || 0)}
                    style={inputStyle}
                    min="0"
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Packet Loss (%):</label>
                  <input
                    type="number"
                    disabled={!config.weak_network_enabled}
                    value={config.loss_rate}
                    onChange={(e) => updateField("loss_rate", parseFloat(e.target.value) || 0)}
                    style={inputStyle}
                    min="0"
                    max="100"
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Bandwidth (Kbps, 0=unlimited):</label>
                  <input
                    type="number"
                    disabled={!config.weak_network_enabled}
                    value={config.bandwidth_kbps}
                    onChange={(e) => updateField("bandwidth_kbps", parseInt(e.target.value) || 0)}
                    style={inputStyle}
                    min="0"
                  />
                </div>
              </div>
            </div>

            {/* UNSTABLE SERVER SIMULATION BLOCK */}
            <div style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-primary)", paddingBottom: "6px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <Server size={14} style={{ color: "var(--text-primary)" }} />
                  <span style={{ fontWeight: 600 }}>Unstable Server Simulation</span>
                </div>
                <input
                  type="checkbox"
                  checked={config.unstable_server_enabled}
                  onChange={(e) => updateField("unstable_server_enabled", e.target.checked)}
                  style={{ cursor: "pointer" }}
                />
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "10px" }}>
                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Drop Rate (%):</label>
                  <input
                    type="number"
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_drop_rate}
                    onChange={(e) => updateField("unstable_server_drop_rate", parseFloat(e.target.value) || 0)}
                    style={inputStyle}
                    min="0"
                    max="100"
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Error Rate (%):</label>
                  <input
                    type="number"
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_error_rate}
                    onChange={(e) => updateField("unstable_server_error_rate", parseFloat(e.target.value) || 0)}
                    style={inputStyle}
                    min="0"
                    max="100"
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Periodic Crash (s, 0=disable):</label>
                  <input
                    type="number"
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_periodic_crash_secs}
                    onChange={(e) => updateField("unstable_server_periodic_crash_secs", parseInt(e.target.value) || 0)}
                    style={inputStyle}
                    min="0"
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>API Error Codes (CSV):</label>
                  <input
                    type="text"
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_error_codes}
                    onChange={(e) => updateField("unstable_server_error_codes", e.target.value)}
                    placeholder="500,502,503"
                    style={inputStyle}
                  />
                </div>
              </div>
            </div>

            {/* RESOURCE LIMITS BLOCK */}
            <div style={cardStyle}>
              <div style={{ display: "flex", alignItems: "center", gap: "6px", borderBottom: "1px solid var(--border-primary)", paddingBottom: "6px" }}>
                <Cpu size={14} style={{ color: "var(--text-primary)" }} />
                <span style={{ fontWeight: 600 }}>Simulated Resource Limits</span>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                <p style={{ fontSize: "10px", color: "var(--text-secondary)", margin: 0 }}>
                  These limits are for sandbox simulation only and are independent of system project configurations.
                </p>

                {/* CPU Limit */}
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "10px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                    <input
                      type="checkbox"
                      checked={config.cpu_limit_enabled}
                      onChange={(e) => updateField("cpu_limit_enabled", e.target.checked)}
                      style={{ cursor: "pointer" }}
                    />
                    <span>CPU Limit (%):</span>
                  </div>
                  <div style={{ width: "100px" }}>
                    <input
                      type="number"
                      disabled={!config.cpu_limit_enabled}
                      value={config.cpu_limit_percent}
                      onChange={(e) => updateField("cpu_limit_percent", Math.max(1, parseInt(e.target.value) || 1))}
                      style={inputStyle}
                      min="1"
                      max="100"
                    />
                  </div>
                </div>

                {/* RAM Limit */}
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "10px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                    <input
                      type="checkbox"
                      checked={config.ram_limit_enabled}
                      onChange={(e) => updateField("ram_limit_enabled", e.target.checked)}
                      style={{ cursor: "pointer" }}
                    />
                    <span>RAM Limit (MB):</span>
                  </div>
                  <div style={{ width: "100px" }}>
                    <input
                      type="number"
                      disabled={!config.ram_limit_enabled}
                      value={config.ram_limit_mb}
                      onChange={(e) => updateField("ram_limit_mb", Math.max(1, parseInt(e.target.value) || 1))}
                      style={inputStyle}
                      min="1"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* CUSTOM ENV VARS & GATEWAY MARKERS BLOCK (MONOCHROME & ENGLISH) */}
          <div style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-primary)", paddingBottom: "8px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <Dna size={14} style={{ color: "var(--text-primary)" }} />
                <span style={{ fontWeight: 600 }}>Env Variables & Gateway Markers Injection</span>
                <span style={{ fontSize: "9px", padding: "2px 6px", borderRadius: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", color: "var(--text-secondary)", border: "1px solid var(--border-primary)", fontWeight: 500 }}>
                  Exposed / Hidden Cloaked
                </span>
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                {/* Presets buttons (Monochrome & English) */}
                <div style={{ display: "flex", gap: "4px" }}>
                  <button
                    type="button"
                    onClick={() => addCustomEnv("X_GATEWAY_TRACE_ID", "trace_id_9981", "hidden", "both")}
                    style={buttonPresetStyle}
                    title="Inject Cloaked Trace ID for both Inbound & Outbound"
                  >
                    <Plus size={10} /> Gateway Trace ID
                  </button>
                  <button
                    type="button"
                    onClick={() => addCustomEnv("X_SIM_AUTH_MARKER", "secret_auth_token", "hidden", "inbound")}
                    style={buttonPresetStyle}
                    title="Inject Cloaked Auth Token for Inbound"
                  >
                    <Plus size={10} /> Auth Token Marker
                  </button>
                  <button
                    type="button"
                    onClick={() => addCustomEnv("NODE_ENV", "simulation", "exposed", "both")}
                    style={buttonPresetStyle}
                    title="Inject Exposed Environment Variable"
                  >
                    <Plus size={10} /> Stage Env
                  </button>
                </div>

                <button
                  type="button"
                  onClick={() => addCustomEnv()}
                  style={{
                    fontSize: "10px",
                    padding: "4px 9px",
                    borderRadius: "3px",
                    border: "1px solid var(--border-primary)",
                    backgroundColor: "var(--text-primary)",
                    color: "var(--bg-primary)",
                    fontWeight: 600,
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    gap: "4px",
                  }}
                >
                  <Plus size={12} /> Add Custom
                </button>

                <input
                  type="checkbox"
                  checked={config.env_injection_enabled}
                  onChange={(e) => updateField("env_injection_enabled", e.target.checked)}
                  style={{ cursor: "pointer", marginLeft: "4px" }}
                />
              </div>
            </div>

            <p style={{ fontSize: "10px", color: "var(--text-secondary)", margin: 0 }}>
              Simulate exposed environment variables injected into the process or hidden cloaked markers auto-injected as HTTP headers via Gateway Proxies.
            </p>

            {config.custom_envs && config.custom_envs.length > 0 ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginTop: "4px" }}>
                {config.custom_envs.map((env) => {
                  return (
                    <div
                      key={env.id}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: "8px",
                        backgroundColor: "var(--bg-secondary, #181818)",
                        padding: "8px 12px",
                        borderRadius: "4px",
                        border: "1px solid var(--border-primary, #2a2a2a)",
                        opacity: env.enabled ? 1 : 0.4,
                        transition: "all 0.2s ease",
                      }}
                    >
                      <input
                        type="checkbox"
                        checked={env.enabled}
                        onChange={(e) => updateCustomEnv(env.id, "enabled", e.target.checked)}
                        style={{ cursor: "pointer" }}
                      />

                      {/* Key Input */}
                      <input
                        type="text"
                        value={env.key}
                        onChange={(e) => updateCustomEnv(env.id, "key", e.target.value)}
                        placeholder="KEY (e.g. SIM_GATEWAY_TOKEN)"
                        style={{ ...inputStyle, width: "190px", fontWeight: 600, color: "var(--text-primary)" }}
                      />

                      <span style={{ fontSize: "11px", color: "var(--text-secondary)" }}>=</span>

                      {/* Value Input */}
                      <input
                        type="text"
                        value={env.value}
                        onChange={(e) => updateCustomEnv(env.id, "value", e.target.value)}
                        placeholder="VALUE"
                        style={{ ...inputStyle, flex: 1 }}
                      />

                      {/* English & Monochrome Icon Pill Toggle for Visibility */}
                      <div style={{ display: "flex", gap: "2px", backgroundColor: "rgba(0,0,0,0.4)", padding: "2px", borderRadius: "4px", border: "1px solid var(--border-primary)" }}>
                        <button
                          type="button"
                          onClick={() => updateCustomEnv(env.id, "visibility", "exposed")}
                          style={{
                            fontSize: "10px",
                            padding: "3px 8px",
                            borderRadius: "3px",
                            border: "none",
                            cursor: "pointer",
                            fontWeight: env.visibility === "exposed" ? 600 : 400,
                            backgroundColor: env.visibility === "exposed" ? "rgba(255, 255, 255, 0.12)" : "transparent",
                            color: env.visibility === "exposed" ? "var(--text-primary)" : "var(--text-secondary)",
                            display: "flex",
                            alignItems: "center",
                            gap: "4px",
                          }}
                        >
                          <Eye size={12} /> Exposed
                        </button>
                        <button
                          type="button"
                          onClick={() => updateCustomEnv(env.id, "visibility", "hidden")}
                          style={{
                            fontSize: "10px",
                            padding: "3px 8px",
                            borderRadius: "3px",
                            border: "none",
                            cursor: "pointer",
                            fontWeight: env.visibility === "hidden" ? 600 : 400,
                            backgroundColor: env.visibility === "hidden" ? "rgba(255, 255, 255, 0.12)" : "transparent",
                            color: env.visibility === "hidden" ? "var(--text-primary)" : "var(--text-secondary)",
                            display: "flex",
                            alignItems: "center",
                            gap: "4px",
                          }}
                        >
                          <Lock size={12} /> Hidden
                        </button>
                      </div>

                      {/* English & Monochrome Icon Segment Buttons for Scope */}
                      <div style={{ display: "flex", gap: "2px", backgroundColor: "rgba(0,0,0,0.4)", padding: "2px", borderRadius: "4px", border: "1px solid var(--border-primary)" }}>
                        <button
                          type="button"
                          onClick={() => updateCustomEnv(env.id, "scope", "both")}
                          title="Both Inbound & Outbound streams"
                          style={{
                            fontSize: "10px",
                            padding: "3px 7px",
                            borderRadius: "3px",
                            border: "none",
                            cursor: "pointer",
                            fontWeight: env.scope === "both" ? 600 : 400,
                            backgroundColor: env.scope === "both" ? "rgba(255, 255, 255, 0.12)" : "transparent",
                            color: env.scope === "both" ? "var(--text-primary)" : "var(--text-secondary)",
                            display: "flex",
                            alignItems: "center",
                            gap: "4px",
                          }}
                        >
                          <TrendingUpDown size={12} /> Both
                        </button>
                        <button
                          type="button"
                          onClick={() => updateCustomEnv(env.id, "scope", "inbound")}
                          title="Inbound Gateway stream only"
                          style={{
                            fontSize: "10px",
                            padding: "3px 7px",
                            borderRadius: "3px",
                            border: "none",
                            cursor: "pointer",
                            fontWeight: env.scope === "inbound" ? 600 : 400,
                            backgroundColor: env.scope === "inbound" ? "rgba(255, 255, 255, 0.12)" : "transparent",
                            color: env.scope === "inbound" ? "var(--text-primary)" : "var(--text-secondary)",
                            display: "flex",
                            alignItems: "center",
                            gap: "4px",
                          }}
                        >
                          <ArrowDown size={12} /> Inbound
                        </button>
                        <button
                          type="button"
                          onClick={() => updateCustomEnv(env.id, "scope", "outbound")}
                          title="Outbound Proxy stream only"
                          style={{
                            fontSize: "10px",
                            padding: "3px 7px",
                            borderRadius: "3px",
                            border: "none",
                            cursor: "pointer",
                            fontWeight: env.scope === "outbound" ? 600 : 400,
                            backgroundColor: env.scope === "outbound" ? "rgba(255, 255, 255, 0.12)" : "transparent",
                            color: env.scope === "outbound" ? "var(--text-primary)" : "var(--text-secondary)",
                            display: "flex",
                            alignItems: "center",
                            gap: "4px",
                          }}
                        >
                          <ArrowUp size={12} /> Outbound
                        </button>
                      </div>

                      {/* Delete Button */}
                      <button
                        type="button"
                        onClick={() => deleteCustomEnv(env.id)}
                        style={{
                          backgroundColor: "transparent",
                          border: "none",
                          color: "var(--text-secondary)",
                          cursor: "pointer",
                          padding: "4px 6px",
                          borderRadius: "3px",
                          display: "flex",
                          alignItems: "center",
                        }}
                        title="Delete variable"
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div style={{ fontSize: "10px", color: "var(--text-secondary)", fontStyle: "italic", textAlign: "center", padding: "12px 0", border: "1px dashed var(--border-primary)", borderRadius: "4px" }}>
                No custom environment variables defined. Click "+ Add Custom" or preset buttons above to add quickly.
              </div>
            )}
          </div>

        </div>
      ) : (
        <div style={{ display: "flex", height: "60%", alignItems: "center", justifyContent: "center", color: "var(--text-secondary)", fontSize: "11px" }}>
          No environment configuration found.
        </div>
      )}
    </div>
  );
}
