import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Project } from "../types";
import { ChevronLeft, ChevronRight } from "lucide-react";

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

interface NumberStepperProps {
  value: number;
  onChange: (val: number) => void;
  disabled?: boolean;
  min?: number;
  max?: number;
  step?: number;
  isFloat?: boolean;
}

function NumberStepper({
  value,
  onChange,
  disabled = false,
  min = 0,
  max,
  step = 1,
  isFloat = false,
}: NumberStepperProps) {
  const [draft, setDraft] = useState<string>(String(value));

  useEffect(() => {
    setDraft(String(value));
  }, [value]);

  const clamp = (n: number) => {
    let res = isNaN(n) ? min : n;
    if (min !== undefined) res = Math.max(min, res);
    if (max !== undefined) res = Math.min(max, res);
    return isFloat ? Math.round(res * 100) / 100 : Math.round(res);
  };

  const commit = (raw: string) => {
    const parsed = isFloat ? parseFloat(raw) : parseInt(raw, 10);
    const next = clamp(parsed);
    setDraft(String(next));
    onChange(next);
  };

  const adjust = (delta: number) => {
    if (disabled) return;
    const next = clamp(value + delta);
    setDraft(String(next));
    onChange(next);
  };

  const btnStyle: React.CSSProperties = {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    width: "24px",
    height: "26px",
    backgroundColor: "rgba(255, 255, 255, 0.03)",
    border: "none",
    color: disabled ? "var(--text-muted, #555)" : "var(--text-secondary, #999)",
    cursor: disabled ? "not-allowed" : "pointer",
    padding: 0,
    flexShrink: 0,
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        backgroundColor: "var(--bg-secondary, #181818)",
        border: "1px solid var(--border-primary, #2a2a2a)",
        borderRadius: "3px",
        overflow: "hidden",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      <button
        type="button"
        disabled={disabled || (min !== undefined && value <= min)}
        onClick={() => adjust(-step)}
        style={{ ...btnStyle, borderRight: "1px solid var(--border-primary, #2a2a2a)" }}
        title="Decrease"
      >
        <ChevronLeft size={12} />
      </button>
      <input
        type="number"
        className="env-sim-num-input"
        disabled={disabled}
        value={draft}
        min={min}
        max={max}
        step={step}
        onChange={(e) => {
          const valStr = e.target.value;
          setDraft(valStr);
          const parsed = isFloat ? parseFloat(valStr) : parseInt(valStr, 10);
          if (!isNaN(parsed)) {
            onChange(clamp(parsed));
          }
        }}
        onBlur={() => commit(draft)}
        style={{
          backgroundColor: "transparent",
          border: "none",
          color: "var(--text-primary, #e0e0e0)",
          padding: "4px 6px",
          fontSize: "11px",
          fontFamily: "var(--font-mono)",
          outline: "none",
          width: "100%",
          textAlign: "center",
          minWidth: 0,
        }}
      />
      <button
        type="button"
        disabled={disabled || (max !== undefined && value >= max)}
        onClick={() => adjust(step)}
        style={{ ...btnStyle, borderLeft: "1px solid var(--border-primary, #2a2a2a)" }}
        title="Increase"
      >
        <ChevronRight size={12} />
      </button>
    </div>
  );
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

    if (saveTimerRef.current) {
      clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
    }

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
          unstable_server_error_codes: rawCfg?.unstable_server_error_codes ?? "500, 502, 503",
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

  useEffect(() => {
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
  }, []);

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
    }, 600);
  };

  const updateField = <K extends keyof EnvSimulationConfig>(key: K, value: EnvSimulationConfig[K]) => {
    if (!config) return;
    let sanitizedVal = value;
    if (typeof value === "number") {
      if (isNaN(value)) {
        sanitizedVal = 0 as EnvSimulationConfig[K];
      } else if (key === "loss_rate" || key === "unstable_server_drop_rate" || key === "unstable_server_error_rate") {
        sanitizedVal = Math.min(100, Math.max(0, value)) as EnvSimulationConfig[K];
      } else if (key === "cpu_limit_percent") {
        sanitizedVal = Math.min(100, Math.max(1, value)) as EnvSimulationConfig[K];
      } else {
        sanitizedVal = Math.max(0, value) as EnvSimulationConfig[K];
      }
    }
    const updated = { ...config, [key]: sanitizedVal };
    setConfig(updated);
    triggerSave(updated);
  };

  const addCustomEnv = (
    presetKey?: string,
    presetVal?: string,
    presetVis?: "exposed" | "hidden",
    presetScope?: "inbound" | "outbound" | "both"
  ) => {
    if (!config) return;
    const newEnv: SimulatedEnvVar = {
      id: "env_" + Date.now() + "_" + Math.floor(Math.random() * 1000),
      key: presetKey ?? "X-Gateway-Marker",
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

  const updateCustomEnv = (id: string, field: keyof SimulatedEnvVar, val: string | boolean) => {
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
    let enabled = config.firewall_enabled;
    if (preset === "local_only") {
      rules = "*, !localhost, !127.0.0.1, !::1, !*.local, !192.168.*, !10.*";
      enabled = true;
    } else if (preset === "block_social") {
      rules = "facebook.com, *.facebook.com, twitter.com, *.twitter.com, x.com, *.x.com, instagram.com, *.instagram.com, tiktok.com, *.tiktok.com, youtube.com, *.youtube.com";
      enabled = true;
    } else if (preset === "block_google") {
      rules = "google.com, *.google.com, googleapis.com, *.googleapis.com, gstatic.com, *.gstatic.com";
      enabled = true;
    } else if (preset === "clear") {
      rules = "";
    }
    const updated = { ...config, firewall_enabled: enabled, firewall_rules: rules };
    setConfig(updated);
    triggerSave(updated);
  };

  if (isLoading) {
    return (
      <div style={{ display: "flex", justifyContent: "center", alignItems: "center", height: "100%", backgroundColor: "var(--bg-primary)" }}>
        <p style={{ color: "var(--text-secondary)", fontSize: "11px", fontFamily: "var(--font-mono)" }}>Loading configuration...</p>
      </div>
    );
  }

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
    fontSize: "10px",
    padding: "3px 8px",
    borderRadius: "3px",
    border: "1px solid var(--border-primary, #2a2a2a)",
    backgroundColor: "rgba(255, 255, 255, 0.04)",
    color: "var(--text-primary, #e0e0e0)",
    cursor: "pointer",
  };

  return (
    <div style={panelStyle}>
      <style>{`
        .env-sim-num-input::-webkit-inner-spin-button,
        .env-sim-num-input::-webkit-outer-spin-button {
          -webkit-appearance: none;
          margin: 0;
        }
        .env-sim-num-input {
          -moz-appearance: textfield;
        }
      `}</style>

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
                <span style={{ fontWeight: 600 }}>Firewall Rules</span>
                <input
                  type="checkbox"
                  checked={config.firewall_enabled}
                  onChange={(e) => updateField("firewall_enabled", e.target.checked)}
                  style={{ cursor: "pointer" }}
                />
              </div>

              <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
                <button type="button" onClick={() => applyFirewallPreset("local_only")} style={buttonPresetStyle}>
                  Local Only
                </button>
                <button type="button" onClick={() => applyFirewallPreset("block_social")} style={buttonPresetStyle}>
                  Block Socials
                </button>
                <button type="button" onClick={() => applyFirewallPreset("block_google")} style={buttonPresetStyle}>
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
                  Blocked domains/IPs (comma separated, prefix ! to allow):
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
                <span style={{ fontWeight: 600 }}>Network Simulation</span>
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
                  <NumberStepper
                    disabled={!config.weak_network_enabled}
                    value={config.latency_ms}
                    onChange={(v) => updateField("latency_ms", v)}
                    min={0}
                    max={60000}
                    step={25}
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Jitter (ms):</label>
                  <NumberStepper
                    disabled={!config.weak_network_enabled}
                    value={config.jitter_ms}
                    onChange={(v) => updateField("jitter_ms", v)}
                    min={0}
                    max={30000}
                    step={10}
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Packet Loss (%):</label>
                  <NumberStepper
                    disabled={!config.weak_network_enabled}
                    value={config.loss_rate}
                    onChange={(v) => updateField("loss_rate", v)}
                    min={0}
                    max={100}
                    step={1}
                    isFloat
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Bandwidth (Kbps, 0=unlimited):</label>
                  <NumberStepper
                    disabled={!config.weak_network_enabled}
                    value={config.bandwidth_kbps}
                    onChange={(v) => updateField("bandwidth_kbps", v)}
                    min={0}
                    step={128}
                  />
                </div>
              </div>
            </div>

            {/* UNSTABLE SERVER SIMULATION BLOCK */}
            <div style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-primary)", paddingBottom: "6px" }}>
                <span style={{ fontWeight: 600 }}>Unstable Server Simulation</span>
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
                  <NumberStepper
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_drop_rate}
                    onChange={(v) => updateField("unstable_server_drop_rate", v)}
                    min={0}
                    max={100}
                    step={5}
                    isFloat
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Error Rate (%):</label>
                  <NumberStepper
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_error_rate}
                    onChange={(v) => updateField("unstable_server_error_rate", v)}
                    min={0}
                    max={100}
                    step={5}
                    isFloat
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Periodic Crash (s, 0=disable):</label>
                  <NumberStepper
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_periodic_crash_secs}
                    onChange={(v) => updateField("unstable_server_periodic_crash_secs", v)}
                    min={0}
                    step={5}
                  />
                </div>

                <div>
                  <label style={{ fontSize: "10px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>API Error Codes (CSV):</label>
                  <input
                    type="text"
                    disabled={!config.unstable_server_enabled}
                    value={config.unstable_server_error_codes}
                    onChange={(e) => updateField("unstable_server_error_codes", e.target.value)}
                    placeholder="500, 502, 503"
                    style={inputStyle}
                  />
                </div>
              </div>
            </div>

            {/* RESOURCE LIMITS BLOCK */}
            <div style={cardStyle}>
              <div style={{ borderBottom: "1px solid var(--border-primary)", paddingBottom: "6px" }}>
                <span style={{ fontWeight: 600 }}>Simulated Resource Limits</span>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                <p style={{ fontSize: "10px", color: "var(--text-secondary)", margin: 0 }}>
                  Watchdog kills the process if CPU or RAM exceeds these limits continuously for 30s.
                </p>

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
                  <div style={{ width: "120px" }}>
                    <NumberStepper
                      disabled={!config.cpu_limit_enabled}
                      value={config.cpu_limit_percent}
                      onChange={(v) => updateField("cpu_limit_percent", v)}
                      min={1}
                      max={100}
                      step={5}
                    />
                  </div>
                </div>

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
                  <div style={{ width: "120px" }}>
                    <NumberStepper
                      disabled={!config.ram_limit_enabled}
                      value={config.ram_limit_mb}
                      onChange={(v) => updateField("ram_limit_mb", v)}
                      min={1}
                      step={128}
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* CUSTOM ENV VARS & GATEWAY MARKERS BLOCK */}
          <div style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-primary)", paddingBottom: "8px", flexWrap: "wrap", gap: "8px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ fontWeight: 600 }}>Env Variables & Gateway Markers Injection</span>
                <span style={{ fontSize: "9px", padding: "2px 6px", borderRadius: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", color: "var(--text-secondary)", border: "1px solid var(--border-primary)", fontWeight: 500 }}>
                  Exposed / Hidden Cloaked
                </span>
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
                <div style={{ display: "flex", gap: "4px" }}>
                  <button
                    type="button"
                    onClick={() => addCustomEnv("X-Gateway-Trace-Id", "trace_id_9981", "hidden", "both")}
                    style={buttonPresetStyle}
                    title="Inject Cloaked Trace ID header for both Inbound & Outbound"
                  >
                    + Gateway Trace ID
                  </button>
                  <button
                    type="button"
                    onClick={() => addCustomEnv("X-Sim-Auth-Marker", "secret_auth_token", "hidden", "inbound")}
                    style={buttonPresetStyle}
                    title="Inject Cloaked Auth Token header for Inbound"
                  >
                    + Auth Token Marker
                  </button>
                  <button
                    type="button"
                    onClick={() => addCustomEnv("NODE_ENV", "simulation", "exposed", "both")}
                    style={buttonPresetStyle}
                    title="Inject Exposed Environment Variable"
                  >
                    + Stage Env
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
                  }}
                >
                  + Add Custom
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
              Exposed variables are injected into process env; Hidden markers are injected as HTTP headers via Gateway Proxies according to scope.
            </p>

            {config.custom_envs && config.custom_envs.length > 0 ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginTop: "4px" }}>
                {config.custom_envs.map((env) => (
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

                    <input
                      type="text"
                      value={env.key}
                      onChange={(e) => updateCustomEnv(env.id, "key", e.target.value)}
                      placeholder="KEY or X-Header-Name"
                      style={{ ...inputStyle, width: "190px", fontWeight: 600, color: "var(--text-primary)" }}
                    />

                    <span style={{ fontSize: "11px", color: "var(--text-secondary)" }}>=</span>

                    <input
                      type="text"
                      value={env.value}
                      onChange={(e) => updateCustomEnv(env.id, "value", e.target.value)}
                      placeholder="VALUE"
                      style={{ ...inputStyle, flex: 1 }}
                    />

                    {/* Visibility Toggle */}
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
                        }}
                      >
                        Exposed
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
                        }}
                      >
                        Hidden
                      </button>
                    </div>

                    {/* Scope Toggle */}
                    <div style={{ display: "flex", gap: "2px", backgroundColor: "rgba(0,0,0,0.4)", padding: "2px", borderRadius: "4px", border: "1px solid var(--border-primary)" }}>
                      {(["both", "inbound", "outbound"] as const).map((sc) => (
                        <button
                          key={sc}
                          type="button"
                          onClick={() => updateCustomEnv(env.id, "scope", sc)}
                          style={{
                            fontSize: "10px",
                            padding: "3px 7px",
                            borderRadius: "3px",
                            border: "none",
                            cursor: "pointer",
                            textTransform: "capitalize",
                            fontWeight: env.scope === sc ? 600 : 400,
                            backgroundColor: env.scope === sc ? "rgba(255, 255, 255, 0.12)" : "transparent",
                            color: env.scope === sc ? "var(--text-primary)" : "var(--text-secondary)",
                          }}
                        >
                          {sc}
                        </button>
                      ))}
                    </div>

                    <button
                      type="button"
                      onClick={() => deleteCustomEnv(env.id)}
                      style={{
                        backgroundColor: "transparent",
                        border: "none",
                        color: "var(--text-secondary)",
                        cursor: "pointer",
                        padding: "2px 6px",
                        borderRadius: "3px",
                        fontSize: "13px",
                        lineHeight: 1,
                      }}
                      title="Delete variable"
                    >
                      ×
                    </button>
                  </div>
                ))}
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
