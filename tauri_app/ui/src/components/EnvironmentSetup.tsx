import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Project } from "../types";

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
    const updated = { ...config, [key]: value };
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

  const inputStyle: React.CSSProperties = {
    backgroundColor: "var(--bg-secondary, #1e1e2e)",
    border: "1px solid var(--border-primary, #313244)",
    color: "var(--text-primary, #cdd6f4)",
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
    backgroundColor: "var(--bg-primary, #11111b)",
    color: "var(--text-primary, #cdd6f4)",
    fontFamily: "var(--font-mono, monospace)",
    fontSize: "12px",
  };

  const headerStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    borderBottom: "1px solid var(--border-primary, #313244)",
    paddingBottom: "12px",
    marginBottom: "16px",
  };

  const cardStyle: React.CSSProperties = {
    border: "1px solid var(--border-primary, #313244)",
    borderRadius: "4px",
    padding: "12px",
    backgroundColor: "transparent",
    display: "flex",
    flexDirection: "column",
    gap: "10px",
  };

  return (
    <div style={panelStyle}>
      {/* Header controls */}
      <div style={headerStyle}>
        <div>
          <h2 style={{ fontSize: "13px", fontWeight: 600, margin: "0 0 2px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
            Environment Simulation Setup
          </h2>
          <p style={{ fontSize: "10px", color: "var(--text-secondary)", margin: 0 }}>
            Configuring custom sandboxed conditions for the selected project.
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
                backgroundColor: "var(--bg-secondary, #1e1e2e)",
                border: "1px solid var(--border-primary, #313244)",
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
          <div style={{ fontSize: "10px" }}>
            {isSaving ? (
              <span style={{ color: "var(--text-secondary)" }}>Saving...</span>
            ) : saveStatus === "success" ? (
              <span style={{ color: "#a6e3a1" }}>Saved</span>
            ) : saveStatus === "error" ? (
              <span style={{ color: "#f38ba8" }}>Error saving</span>
            ) : (
              <span style={{ color: "var(--text-secondary)" }}>Auto-save</span>
            )}
          </div>
        </div>
      </div>

      {config ? (
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
            
            <div style={{ display: "flex", flexDirection: "column", gap: "8px", opacity: config.firewall_enabled ? 1 : 0.4 }}>
              <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
                <button
                  disabled={!config.firewall_enabled}
                  onClick={() => applyFirewallPreset("local_only")}
                  style={{ fontSize: "9px", padding: "2px 6px", backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-primary)", color: "var(--text-primary)", cursor: "pointer", borderRadius: "2px" }}
                >
                  Local Only
                </button>
                <button
                  disabled={!config.firewall_enabled}
                  onClick={() => applyFirewallPreset("block_social")}
                  style={{ fontSize: "9px", padding: "2px 6px", backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-primary)", color: "var(--text-primary)", cursor: "pointer", borderRadius: "2px" }}
                >
                  Block Socials
                </button>
                <button
                  disabled={!config.firewall_enabled}
                  onClick={() => applyFirewallPreset("block_google")}
                  style={{ fontSize: "9px", padding: "2px 6px", backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-primary)", color: "var(--text-primary)", cursor: "pointer", borderRadius: "2px" }}
                >
                  Block Google
                </button>
                <button
                  disabled={!config.firewall_enabled}
                  onClick={() => applyFirewallPreset("clear")}
                  style={{ fontSize: "9px", padding: "2px 6px", backgroundColor: "rgba(243, 139, 168, 0.1)", border: "1px solid #f38ba8", color: "#f38ba8", cursor: "pointer", borderRadius: "2px" }}
                >
                  Clear
                </button>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Blocked domains/IPs (comma separated):</label>
                <textarea
                  disabled={!config.firewall_enabled}
                  placeholder="e.g. *.google.com, github.com, 1.1.1.1"
                  value={config.firewall_rules}
                  onChange={(e) => updateField("firewall_rules", e.target.value)}
                  style={{
                    backgroundColor: "var(--bg-secondary)",
                    border: "1px solid var(--border-primary)",
                    borderRadius: "3px",
                    color: "var(--text-primary)",
                    padding: "6px",
                    fontSize: "11px",
                    fontFamily: "var(--font-mono)",
                    outline: "none",
                    resize: "none",
                    height: "100px",
                  }}
                />
              </div>
            </div>
          </div>

          {/* WEAK NETWORK BLOCK */}
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

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "10px", opacity: config.weak_network_enabled ? 1 : 0.4 }}>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Latency (ms):</label>
                <input
                  type="number"
                  disabled={!config.weak_network_enabled}
                  value={config.latency_ms}
                  onChange={(e) => updateField("latency_ms", Math.max(0, parseInt(e.target.value) || 0))}
                  style={inputStyle}
                  min="0"
                />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Jitter (ms):</label>
                <input
                  type="number"
                  disabled={!config.weak_network_enabled}
                  value={config.jitter_ms}
                  onChange={(e) => updateField("jitter_ms", Math.max(0, parseInt(e.target.value) || 0))}
                  style={inputStyle}
                  min="0"
                />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Packet Loss (%):</label>
                <input
                  type="number"
                  step="0.1"
                  disabled={!config.weak_network_enabled}
                  value={config.loss_rate}
                  onChange={(e) => updateField("loss_rate", Math.min(100, Math.max(0, parseFloat(e.target.value) || 0)))}
                  style={inputStyle}
                  min="0"
                  max="100"
                />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Bandwidth (Kbps, 0=unlimited):</label>
                <input
                  type="number"
                  disabled={!config.weak_network_enabled}
                  value={config.bandwidth_kbps}
                  onChange={(e) => updateField("bandwidth_kbps", Math.max(0, parseInt(e.target.value) || 0))}
                  style={inputStyle}
                  min="0"
                />
              </div>
            </div>
          </div>

          {/* UNSTABLE SERVER BLOCK */}
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

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "10px", opacity: config.unstable_server_enabled ? 1 : 0.4 }}>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Drop Rate (%):</label>
                <input
                  type="number"
                  step="0.1"
                  disabled={!config.unstable_server_enabled}
                  value={config.unstable_server_drop_rate}
                  onChange={(e) => updateField("unstable_server_drop_rate", Math.min(100, Math.max(0, parseFloat(e.target.value) || 0)))}
                  style={inputStyle}
                  min="0"
                  max="100"
                />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Error Rate (%):</label>
                <input
                  type="number"
                  step="0.1"
                  disabled={!config.unstable_server_enabled}
                  value={config.unstable_server_error_rate}
                  onChange={(e) => updateField("unstable_server_error_rate", Math.min(100, Math.max(0, parseFloat(e.target.value) || 0)))}
                  style={inputStyle}
                  min="0"
                  max="100"
                />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>Periodic Crash (s, 0=disable):</label>
                <input
                  type="number"
                  disabled={!config.unstable_server_enabled}
                  value={config.unstable_server_periodic_crash_secs}
                  onChange={(e) => updateField("unstable_server_periodic_crash_secs", Math.max(0, parseInt(e.target.value) || 0))}
                  style={inputStyle}
                  min="0"
                />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                <label style={{ fontSize: "10px", color: "var(--text-secondary)" }}>API Error Codes (CSV):</label>
                <input
                  type="text"
                  disabled={!config.unstable_server_enabled}
                  value={config.unstable_server_error_codes}
                  onChange={(e) => updateField("unstable_server_error_codes", e.target.value)}
                  style={inputStyle}
                />
              </div>
            </div>
          </div>

          {/* SIMULATED PERFORMANCE LIMITS BLOCK */}
          <div style={cardStyle}>
            <div style={{ borderBottom: "1px solid var(--border-primary)", paddingBottom: "6px" }}>
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
      ) : (
        <div style={{ display: "flex", height: "60%", alignItems: "center", justifyContent: "center", color: "var(--text-secondary)", fontSize: "11px" }}>
          No environment configuration found.
        </div>
      )}
    </div>
  );
}
