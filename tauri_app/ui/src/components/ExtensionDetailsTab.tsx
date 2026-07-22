import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Box, Check, Settings, ShieldCheck, FileText, Lock, Globe, ExternalLink, AlertTriangle } from "lucide-react";

interface PublisherInfo {
  id: string;
  name: string;
  public_key?: string;
  verified?: boolean;
}

interface ExtensionManifest {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  publisher?: PublisherInfo;
  icon?: string;
  repository?: string;
  readme_url?: string;
  sha256?: string;
  signature?: string;
  capabilities?: {
    permissions?: string[];
  };
}

export default function ExtensionDetailsTab({ extensionId }: { extensionId: string }) {
  const [ext, setExt] = useState<ExtensionManifest | null>(null);
  const [readmeContent, setReadmeContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      try {
        const data = await invoke<ExtensionManifest>("get_extension_details", { id: extensionId });
        setExt(data);
        if (data?.readme_url) {
          try {
            const resp = await fetch(data.readme_url);
            if (resp.ok) {
              const text = await resp.text();
              setReadmeContent(text);
            }
          } catch (e) {
            console.error("Failed to fetch README:", e);
          }
        }
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

  const permissions = ext.capabilities?.permissions || [];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)", overflowY: "auto" }}>
      {/* Header section */}
      <div style={{ display: "flex", gap: "24px", padding: "40px", borderBottom: "1px solid var(--border-primary)", backgroundColor: "var(--bg-secondary)" }}>
        {ext.icon ? (
          <img 
            src={ext.icon} 
            alt={ext.name} 
            style={{ width: "96px", height: "96px", borderRadius: "16px", objectFit: "cover", boxShadow: "0 4px 16px rgba(0,0,0,0.2)" }} 
          />
        ) : (
          <div style={{ width: "96px", height: "96px", backgroundColor: "#8b5cf6", borderRadius: "16px", display: "flex", alignItems: "center", justifyContent: "center", boxShadow: "0 4px 16px rgba(0,0,0,0.2)" }}>
            <Box size={48} color="white" />
          </div>
        )}

        <div style={{ flex: 1 }}>
          <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "6px" }}>
            <h1 style={{ margin: 0, fontSize: "28px", fontWeight: 700 }}>
              {ext.name}
            </h1>
            <span style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-tertiary)", backgroundColor: "var(--bg-tertiary)", padding: "3px 10px", borderRadius: "12px" }}>
              v{ext.version}
            </span>
          </div>

          <div style={{ fontSize: "12px", color: "#8b5cf6", fontFamily: "monospace", marginBottom: "12px" }}>
            {ext.id}
          </div>

          <div style={{ fontSize: "14px", color: "var(--text-secondary)", marginBottom: "16px" }}>
            {ext.description || "No description provided."}
          </div>

          <div style={{ display: "flex", gap: "16px", alignItems: "center", flexWrap: "wrap" }}>
            <span style={{ display: "flex", alignItems: "center", gap: "6px", fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>
              {ext.publisher?.name || ext.author || "Community Developer"}
              {ext.publisher?.verified && (
                <span title="Verified Publisher with Ed25519 Cryptographic Signature" style={{ color: "#3b82f6", display: "flex", alignItems: "center", gap: "2px", fontSize: "11px", fontWeight: 600, backgroundColor: "rgba(59, 130, 246, 0.1)", padding: "2px 8px", borderRadius: "10px" }}>
                  <ShieldCheck size={14} color="#3b82f6" /> Verified Publisher
                </span>
              )}
            </span>

            {ext.sha256 && (
              <span style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "11px", color: "#10b981", backgroundColor: "rgba(16, 185, 129, 0.1)", padding: "2px 8px", borderRadius: "10px", fontWeight: 600 }}>
                <Check size={12} /> SHA-256 Verified
              </span>
            )}
          </div>

          <div style={{ display: "flex", gap: "12px", marginTop: "24px" }}>
            <button style={{ padding: "8px 20px", backgroundColor: "#ef4444", color: "white", border: "none", borderRadius: "6px", cursor: "pointer", fontWeight: 600, fontSize: "13px" }}>
              Uninstall
            </button>
            <button style={{ padding: "8px 20px", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-primary)", borderRadius: "6px", cursor: "pointer", display: "flex", alignItems: "center", gap: "6px", fontSize: "13px", fontWeight: 500 }}>
              <Settings size={14} /> Configure Settings
            </button>
            {ext.repository && (
              <a 
                href={ext.repository} 
                target="_blank" 
                rel="noreferrer"
                style={{ padding: "8px 16px", backgroundColor: "transparent", color: "var(--text-secondary)", border: "1px solid var(--border-primary)", borderRadius: "6px", textDecoration: "none", display: "flex", alignItems: "center", gap: "6px", fontSize: "13px" }}>
                <Globe size={14} /> Repository <ExternalLink size={12} />
              </a>
            )}
          </div>
        </div>
      </div>

      {/* Body Section */}
      <div style={{ padding: "40px", maxWidth: "900px", display: "flex", flexDirection: "column", gap: "32px" }}>
        
        {/* Permission Firewall Section */}
        <div style={{ backgroundColor: "var(--bg-secondary)", padding: "20px", borderRadius: "10px", border: "1px solid var(--border-primary)" }}>
          <h3 style={{ margin: "0 0 12px 0", fontSize: "15px", display: "flex", alignItems: "center", gap: "8px" }}>
            <Lock size={16} color="#eab308" /> Security & Permission Firewall
          </h3>
          {permissions.length === 0 ? (
            <div style={{ fontSize: "13px", color: "#10b981", display: "flex", alignItems: "center", gap: "6px" }}>
              <ShieldCheck size={16} /> Zero-Trust Security: This WASM extension runs with 0 system permissions.
            </div>
          ) : (
            <div>
              <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "10px" }}>
                This extension has been granted the following isolated permissions by the Host Interceptor:
              </div>
              <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
                {permissions.map((p) => (
                  <span key={p} style={{ fontSize: "12px", color: "#eab308", backgroundColor: "rgba(234, 179, 8, 0.1)", border: "1px solid rgba(234, 179, 8, 0.3)", padding: "4px 10px", borderRadius: "6px", fontFamily: "monospace", display: "flex", alignItems: "center", gap: "4px" }}>
                    <AlertTriangle size={12} /> {p}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Documentation / README section */}
        <div>
          <h2 style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "18px", borderBottom: "1px solid var(--border-primary)", paddingBottom: "12px", marginBottom: "20px" }}>
            <FileText size={18} /> Documentation & Overview
          </h2>
          {readmeContent ? (
            <div style={{ lineHeight: 1.6, color: "var(--text-secondary)", whiteSpace: "pre-wrap", fontFamily: "sans-serif", backgroundColor: "var(--bg-secondary)", padding: "20px", borderRadius: "8px" }}>
              {readmeContent}
            </div>
          ) : (
            <div style={{ lineHeight: 1.6, color: "var(--text-secondary)" }}>
              <p>Welcome to <strong>{ext.name}</strong> (<code>{ext.id}</code>).</p>
              <p>This extension runs inside an isolated Wasmtime WebAssembly sandbox with strict memory boundaries and Host API interceptor protection.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
