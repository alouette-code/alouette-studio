import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Box, Check, Settings, Shield, FileText } from "lucide-react";

interface ExtensionManifest {
  id: String;
  name: String;
  version: String;
  description?: String;
  author?: String;
}

export default function ExtensionDetailsTab({ extensionId }: { extensionId: string }) {
  const [ext, setExt] = useState<ExtensionManifest | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      try {
        const data = await invoke<ExtensionManifest>("get_extension_details", { id: extensionId });
        setExt(data);
      } catch (err) {
        console.error("Failed to load extension:", err);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [extensionId]);

  if (loading) {
    return <div style={{ padding: "40px", color: "var(--text-secondary)" }}>Loading extension details...</div>;
  }

  if (!ext) {
    return <div style={{ padding: "40px", color: "var(--color-danger)" }}>Extension not found: {extensionId}</div>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)", overflowY: "auto" }}>
      {/* Header section */}
      <div style={{ display: "flex", gap: "20px", padding: "40px", borderBottom: "1px solid var(--border-primary)", backgroundColor: "var(--bg-secondary)" }}>
        <div style={{ width: "96px", height: "96px", backgroundColor: "var(--color-accent)", borderRadius: "12px", display: "flex", alignItems: "center", justifyContent: "center", boxShadow: "0 4px 12px rgba(0,0,0,0.1)" }}>
          <Box size={48} color="white" />
        </div>
        <div style={{ flex: 1 }}>
          <h1 style={{ margin: "0 0 8px 0", fontSize: "28px", display: "flex", alignItems: "center", gap: "12px" }}>
            {ext.name}
            <span style={{ fontSize: "14px", fontWeight: "normal", color: "var(--text-tertiary)", backgroundColor: "var(--bg-tertiary)", padding: "2px 8px", borderRadius: "12px" }}>v{ext.version}</span>
          </h1>
          <div style={{ fontSize: "14px", color: "var(--text-secondary)", marginBottom: "16px" }}>
            {ext.description || "No description provided."}
          </div>
          <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
            <span style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "13px", color: "var(--text-secondary)" }}>
              <Shield size={14} /> By {ext.author || "Unknown"}
            </span>
            <span style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "13px", color: "var(--text-tertiary)" }}>
              <Check size={14} color="var(--color-success)" /> Installed
            </span>
          </div>
          <div style={{ display: "flex", gap: "10px", marginTop: "24px" }}>
            <button style={{ padding: "6px 16px", backgroundColor: "var(--color-danger)", color: "white", border: "none", borderRadius: "4px", cursor: "pointer", fontWeight: 600 }}>
              Uninstall
            </button>
            <button style={{ padding: "6px 16px", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-primary)", borderRadius: "4px", cursor: "pointer", display: "flex", alignItems: "center", gap: "6px" }}>
              <Settings size={14} /> Configure
            </button>
          </div>
        </div>
      </div>

      {/* Body section (README mock) */}
      <div style={{ padding: "40px", maxWidth: "800px" }}>
        <h2 style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "18px", borderBottom: "1px solid var(--border-primary)", paddingBottom: "12px", marginBottom: "20px" }}>
          <FileText size={18} /> Extension Overview
        </h2>
        <div style={{ lineHeight: 1.6, color: "var(--text-secondary)" }}>
          <p>This is the detail page for <strong>{ext.name}</strong>.</p>
          <p>Features and capabilities of this extension will be documented here. You can configure the specific permissions and manage the IPC (Inter-Process Communication) connection to this plugin from this panel.</p>
        </div>
      </div>
    </div>
  );
}
