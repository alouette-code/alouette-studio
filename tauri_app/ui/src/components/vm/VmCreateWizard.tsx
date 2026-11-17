import { useMemo, useState } from "react";
import { ArrowRight, Check, FolderOpen, Plus, Save } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { AlouetteCheckbox, AlouetteNumberInput, AlouetteRadioGroup } from "../AlouetteControls";
import { AlouetteSelect, SelectOption } from "../AlouetteSelect";
import { OS_FAMILIES } from "./presets";
import { VmConfig, VmNotice } from "./types";

interface VmCreateWizardProps {
  onCreated: (created: VmConfig) => void;
  onNotice: (notice: VmNotice) => void;
}

export function VmCreateWizard({ onCreated, onNotice }: VmCreateWizardProps) {
  const [wizardStep, setWizardStep] = useState<1 | 2 | 3 | 4>(1);
  const [osFamily, setOsFamily] = useState<string>("linux");
  const [osType, setOsType] = useState<string>("ubuntu");
  const [vmName, setVmName] = useState("New-VM");
  const [vmDir, setVmDir] = useState("");
  const [cpuCores, setCpuCores] = useState(2);
  const [ramMb, setRamMb] = useState(4096);
  const [arch, setArch] = useState<"x86_64" | "aarch64">("x86_64");
  const [tpmEnabled, setTpmEnabled] = useState(false);
  const [networkType, setNetworkType] = useState("nat");
  const [isoPath, setIsoPath] = useState("");
  const [diskSizeGb, setDiskSizeGb] = useState(30);
  const [firmware, setFirmware] = useState<"bios" | "uefi">("bios");
  const [creating, setCreating] = useState(false);

  const applyOsTemplate = (familyKey: string, versionId: string) => {
    setOsType(versionId);
    const ver = OS_FAMILIES[familyKey]?.versions.find((v) => v.id === versionId);
    if (ver) {
      setCpuCores(ver.conf.cpu);
      setRamMb(ver.conf.ramMb);
      setDiskSizeGb(ver.conf.diskGb);
      setFirmware(ver.conf.fw);
      setArch(ver.conf.arch ?? "x86_64");
      setTpmEnabled(ver.conf.tpm ?? false);
    }
  };

  const handleFamilyChange = (familyKey: string) => {
    setOsFamily(familyKey);
    const firstVersion = OS_FAMILIES[familyKey]?.versions[0];
    if (firstVersion) {
      applyOsTemplate(familyKey, firstVersion.id);
    }
  };

  const familyOptions: SelectOption[] = useMemo(
    () =>
      Object.entries(OS_FAMILIES).map(([key, val]) => ({
        value: key,
        label: val.label,
        badge: `${val.versions.length}`,
      })),
    []
  );

  const versionOptions: SelectOption[] = useMemo(
    () =>
      (OS_FAMILIES[osFamily]?.versions ?? []).map((v) => ({
        value: v.id,
        label: v.label,
        description: v.desc,
      })),
    [osFamily]
  );

  const archOptions: SelectOption[] = useMemo(
    () => [
      { value: "x86_64", label: "x86_64 (PC / KVM)" },
      { value: "aarch64", label: "ARM64 (aarch64 virt)" },
    ],
    []
  );

  const networkOptions: SelectOption[] = useMemo(
    () => [
      { value: "nat", label: "NAT (User Mode) - Default" },
      { value: "bridge", label: "Bridged Adapter (br0)" },
      { value: "host-only", label: "Host-Only / Isolated" },
    ],
    []
  );

  const browseFile = async (setter: (val: string) => void) => {
    try {
      const path = await invoke<string | null>("open_file_dialog");
      if (path) setter(path);
    } catch {
      // cancelled
    }
  };

  const browseFolder = async (setter: (val: string) => void) => {
    try {
      const path = await invoke<string | null>("open_folder_dialog");
      if (path) setter(path);
    } catch {
      // cancelled
    }
  };

  const handleCreateVm = async () => {
    if (!vmName.trim()) {
      onNotice({ type: "error", message: "VM name cannot be empty." });
      setWizardStep(1);
      return;
    }

    setCreating(true);
    try {
      const config: VmConfig = {
        id: "",
        name: vmName.trim(),
        os_type: osType,
        arch,
        cpu_cores: Math.max(1, Math.min(64, cpuCores)),
        ram_size_mb: Math.max(128, Math.min(65536, ramMb)),
        vm_dir: vmDir.trim(),
        iso_path: isoPath.trim() || null,
        disk_path: null,
        disk_size_gb: Math.max(1, Math.min(2000, diskSizeGb)),
        network_mode: networkType,
        firmware,
        advanced: {
          audio_enabled: false,
          tpm_enabled: tpmEnabled,
          graceful_shutdown_timeout: 30,
          host_port_forwards: [],
          additional_disks: [],
        },
      };
      const saved = await invoke<VmConfig>("save_virtual_machine", { config });
      onNotice({ type: "success", message: `VM "${saved.name}" created.` });
      onCreated(saved);
    } catch (err) {
      onNotice({ type: "error", message: `Failed to create VM: ${err}` });
    } finally {
      setCreating(false);
    }
  };

  const selectedVersion = OS_FAMILIES[osFamily]?.versions.find((v) => v.id === osType);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "32px", alignItems: "center", overflowY: "auto" }}>
      <div style={{ width: "100%", maxWidth: "640px" }}>
        <h2 style={{ fontSize: "20px", marginBottom: "8px", display: "flex", alignItems: "center", gap: "8px" }}>
          <Plus size={20} color="var(--color-accent)" /> Create New Virtual Machine
        </h2>
        <p style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "24px" }}>
          Configure a QEMU/KVM virtual machine (x86_64 &amp; ARM64, UEFI + TPM 2.0 supported).
        </p>

        <div style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-primary)", borderRadius: "4px", overflow: "hidden" }}>
          <div style={{ display: "flex", backgroundColor: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-primary)" }}>
            {[
              { step: 1, label: "OS Template" },
              { step: 2, label: "Hardware" },
              { step: 3, label: "Storage" },
              { step: 4, label: "Summary" },
            ].map((s) => (
              <div
                key={s.step}
                onClick={() => setWizardStep(s.step as 1 | 2 | 3 | 4)}
                style={{
                  flex: 1,
                  padding: "12px 8px",
                  textAlign: "center",
                  fontSize: "12px",
                  fontWeight: 700,
                  cursor: "pointer",
                  color: wizardStep >= s.step ? "var(--color-accent)" : "var(--text-muted)",
                  borderBottom: wizardStep === s.step ? "2px solid var(--color-accent)" : "2px solid transparent",
                }}
              >
                Step {s.step}: {s.label}
              </div>
            ))}
          </div>

          <div style={{ padding: "24px", minHeight: "310px" }}>
            {wizardStep === 1 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
                <div>
                  <label className="vm-label">Virtual Machine Name</label>
                  <input
                    type="text"
                    value={vmName}
                    onChange={(e) => setVmName(e.target.value)}
                    className="vm-input"
                    style={{ fontSize: "14px", padding: "8px" }}
                  />
                </div>

                <div>
                  <label className="vm-label">Operating System Type</label>
                  <p className="vm-hint" style={{ marginBottom: "12px" }}>
                    Selecting an OS auto-configures CPU, RAM, UEFI, and TPM 2.0 presets.
                  </p>

                  <div style={{ display: "flex", gap: "12px", marginBottom: "16px" }}>
                    <div style={{ flex: 1 }}>
                      <label className="vm-label">OS Family</label>
                      <AlouetteSelect
                        value={osFamily}
                        options={familyOptions}
                        onChange={handleFamilyChange}
                        searchable={false}
                      />
                    </div>
                    <div style={{ flex: 1 }}>
                      <label className="vm-label">Distribution / Version</label>
                      <AlouetteSelect
                        value={osType}
                        options={versionOptions}
                        onChange={(val) => applyOsTemplate(osFamily, val)}
                        searchable={versionOptions.length > 5}
                        searchPlaceholder="Search distro / version..."
                      />
                    </div>
                  </div>

                  <div style={{ padding: "12px", backgroundColor: "var(--bg-tertiary)", border: "1px solid var(--border-primary)", borderRadius: "4px" }}>
                    <div style={{ fontSize: "12px", fontWeight: 700, color: "var(--color-accent)", marginBottom: "4px" }}>
                      Recommended Preset:
                    </div>
                    <div className="vm-hint">{selectedVersion?.desc}</div>
                  </div>
                </div>
              </div>
            )}

            {wizardStep === 2 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "18px" }}>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
                  <div>
                    <label className="vm-label">CPU Cores</label>
                    <AlouetteNumberInput
                      min={1}
                      max={64}
                      value={cpuCores}
                      onChange={setCpuCores}
                    />
                  </div>
                  <div>
                    <label className="vm-label">RAM (MB)</label>
                    <AlouetteNumberInput
                      min={128}
                      max={65536}
                      step={256}
                      unit="MB"
                      value={ramMb}
                      onChange={setRamMb}
                    />
                    <div style={{ display: "flex", gap: "4px", marginTop: "6px" }}>
                      {[512, 1024, 2048, 4096, 8192].map((preset) => (
                        <button
                          key={preset}
                          type="button"
                          onClick={() => setRamMb(preset)}
                          className="vm-btn"
                          style={{ padding: "2px 6px", fontSize: "10px" }}
                        >
                          {preset >= 1024 ? `${preset / 1024}G` : `${preset}M`}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
                  <div>
                    <label className="vm-label">CPU Architecture</label>
                    <AlouetteSelect
                      value={arch}
                      options={archOptions}
                      onChange={(val) => setArch(val as "x86_64" | "aarch64")}
                      searchable={false}
                    />
                  </div>
                  <div>
                    <label className="vm-label">Firmware Interface</label>
                    <div style={{ marginTop: "6px" }}>
                      <AlouetteRadioGroup
                        value={firmware}
                        onChange={(val) => setFirmware(val as "bios" | "uefi")}
                        options={[
                          { value: "bios", label: "BIOS" },
                          { value: "uefi", label: "UEFI (OVMF)" },
                        ]}
                      />
                    </div>
                  </div>
                </div>

                <div>
                  <AlouetteCheckbox
                    checked={tpmEnabled}
                    onChange={setTpmEnabled}
                    label="Enable Virtual TPM 2.0 (swtpm — required for Windows 11)"
                  />
                </div>
              </div>
            )}

            {wizardStep === 3 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
                <div>
                  <label className="vm-label">Disk Size (GB)</label>
                  <AlouetteNumberInput
                    min={1}
                    max={2000}
                    unit="GB"
                    value={diskSizeGb}
                    onChange={setDiskSizeGb}
                  />
                  <p className="vm-hint" style={{ marginTop: "4px" }}>
                    A sparse .qcow2 disk image will be created on demand.
                  </p>
                </div>

                <div>
                  <label className="vm-label">Installer Image (ISO)</label>
                  <div style={{ display: "flex", gap: "4px" }}>
                    <input
                      type="text"
                      value={isoPath}
                      onChange={(e) => setIsoPath(e.target.value)}
                      className="vm-input"
                      placeholder="Optional. Select an ISO to boot from."
                    />
                    <button type="button" onClick={() => browseFile(setIsoPath)} className="vm-btn">
                      <FolderOpen size={12} />
                    </button>
                  </div>
                </div>

                <div>
                  <label className="vm-label">Network Adapter</label>
                  <AlouetteSelect
                    value={networkType}
                    options={networkOptions}
                    onChange={setNetworkType}
                    searchable={false}
                  />
                </div>

                <div>
                  <label className="vm-label">VM Directory (Location)</label>
                  <div style={{ display: "flex", gap: "4px" }}>
                    <input
                      type="text"
                      value={vmDir}
                      onChange={(e) => setVmDir(e.target.value)}
                      className="vm-input"
                      placeholder="Leave empty for default managed location"
                    />
                    <button type="button" onClick={() => browseFolder(setVmDir)} className="vm-btn">
                      <FolderOpen size={12} />
                    </button>
                  </div>
                </div>
              </div>
            )}

            {wizardStep === 4 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                <div
                  style={{
                    backgroundColor: "var(--bg-primary)",
                    padding: "16px",
                    borderRadius: "4px",
                    border: "1px solid var(--border-primary)",
                    fontFamily: "var(--font-mono, monospace)",
                    fontSize: "12px",
                  }}
                >
                  <div style={{ marginBottom: "8px" }}><strong style={{ color: "var(--color-accent)" }}>Name:</strong> {vmName}</div>
                  <div style={{ marginBottom: "8px" }}><strong style={{ color: "var(--color-accent)" }}>OS Profile:</strong> {selectedVersion?.label} ({arch})</div>
                  <div style={{ marginBottom: "8px" }}><strong style={{ color: "var(--color-accent)" }}>Hardware:</strong> {cpuCores} Cores, {ramMb} MB RAM</div>
                  <div style={{ marginBottom: "8px" }}><strong style={{ color: "var(--color-accent)" }}>Firmware / Security:</strong> {firmware.toUpperCase()} {tpmEnabled ? "+ TPM 2.0" : ""}</div>
                  <div style={{ marginBottom: "8px" }}><strong style={{ color: "var(--color-accent)" }}>Storage:</strong> {diskSizeGb} GB (.qcow2)</div>
                  <div style={{ marginBottom: "8px" }}><strong style={{ color: "var(--color-accent)" }}>Installer ISO:</strong> {isoPath || "None"}</div>
                  <div><strong style={{ color: "var(--color-accent)" }}>Network:</strong> {networkType}</div>
                </div>
                <p style={{ fontSize: "12px", color: "var(--color-success)", display: "flex", alignItems: "center", gap: "6px", fontWeight: 700 }}>
                  <Check size={14} /> Ready to Create
                </p>
              </div>
            )}
          </div>

          <div
            style={{
              padding: "16px 24px",
              borderTop: "1px solid var(--border-primary)",
              display: "flex",
              justifyContent: "space-between",
              backgroundColor: "var(--bg-primary)",
            }}
          >
            <button
              type="button"
              onClick={() => setWizardStep((prev) => Math.max(1, prev - 1) as 1 | 2 | 3 | 4)}
              disabled={wizardStep === 1 || creating}
              className="vm-btn"
              style={{ width: "100px", justifyContent: "center" }}
            >
              Back
            </button>

            {wizardStep < 4 ? (
              <button
                type="button"
                onClick={() => setWizardStep((prev) => Math.min(4, prev + 1) as 1 | 2 | 3 | 4)}
                className="vm-btn vm-btn-primary"
                style={{ width: "100px", justifyContent: "center" }}
              >
                Next <ArrowRight size={14} />
              </button>
            ) : (
              <button
                type="button"
                onClick={handleCreateVm}
                disabled={creating}
                className="vm-btn vm-btn-primary"
                style={{
                  width: "150px",
                  justifyContent: "center",
                  backgroundColor: "var(--color-success)",
                  borderColor: "var(--color-success)",
                }}
              >
                <Save size={14} /> {creating ? "Creating..." : "Finish & Create"}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
