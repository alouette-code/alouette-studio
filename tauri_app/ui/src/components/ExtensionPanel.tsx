import { useState, useEffect } from "react";
import { Search, Settings, Box, Loader2, UploadCloud, ShieldCheck, Download } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

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
  sha256?: string;
  capabilities?: {
    permissions?: string[];
  };
}

interface RegistryIndexItem {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  publisher?: PublisherInfo;
  icon?: string;
  repository: string;
  wasm_url: string;
  sha256: string;
  permissions: string[];
}

interface ExtensionPanelProps {
  onFileSelect?: (path: string) => void;
}

export default function ExtensionPanel({ onFileSelect }: ExtensionPanelProps) {
  const [activeTab, setActiveTab] = useState<"installed" | "marketplace">("installed");
  const [searchQuery, setSearchQuery] = useState("");
  const [installedExts, setInstalledExts] = useState<ExtensionManifest[]>([]);
  const [marketplaceExts, setMarketplaceExts] = useState<RegistryIndexItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [installingId, setInstallingId] = useState<string | null>(null);

  const fetchExtensions = async () => {
    setLoading(true);
    try {
      if (activeTab === "installed") {
        const data = await invoke<ExtensionManifest[]>("get_installed_extensions");
        setInstalledExts(data);
      } else {
        const data = await invoke<RegistryIndexItem[]>("fetch_marketplace_extensions");
        setMarketplaceExts(data);
      }
    } catch (err) {
      console.error("Failed to load extensions:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchExtensions();
  }, [activeTab]);

  const handleInstall = async (item: RegistryIndexItem, e: React.MouseEvent) => {
    e.stopPropagation();
    setInstallingId(item.id);
    try {
      await invoke("install_wasm_extension", { item });
      alert(`Successfully installed WASM extension: ${item.name}`);
      setActiveTab("installed");
    } catch (err) {
      alert(`Installation failed: ${err}`);
    } finally {
      setInstallingId(null);
    }
  };

  return (
    <div className="extension-panel" style={{ display: "flex", flexDirection: "column", height: "100%", backgroundColor: "var(--bg-secondary)" }}>
      {/* Header & Search */}
      <div style={{ padding: "12px", borderBottom: "1px solid var(--border-primary)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "10px" }}>
          <h3 style={{ margin: 0, fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px" }}>
            Wasm Extensions
          </h3>
          <button 
            title="Publish New Extension"
            onClick={() => onFileSelect?.("__publish_extension__")}
            style={{ background: "none", border: "none", color: "var(--text-tertiary)", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center" }}
          >
            <UploadCloud size={14} />
          </button>
        </div>

        <div style={{ display: "flex", gap: "2px", marginBottom: "10px", backgroundColor: "var(--bg-tertiary)", padding: "2px", borderRadius: "6px" }}>
          <button 
            onClick={() => setActiveTab("installed")}
            style={{ flex: 1, padding: "4px 0", fontSize: "11px", fontWeight: activeTab === "installed" ? 600 : "normal", backgroundColor: activeTab === "installed" ? "var(--bg-secondary)" : "transparent", color: activeTab === "installed" ? "var(--text-primary)" : "var(--text-secondary)", border: "none", borderRadius: "4px", cursor: "pointer", boxShadow: activeTab === "installed" ? "0 1px 3px rgba(0,0,0,0.1)" : "none" }}
          >
            Installed
          </button>
          <button 
            onClick={() => setActiveTab("marketplace")}
            style={{ flex: 1, padding: "4px 0", fontSize: "11px", fontWeight: activeTab === "marketplace" ? 600 : "normal", backgroundColor: activeTab === "marketplace" ? "var(--bg-secondary)" : "transparent", color: activeTab === "marketplace" ? "var(--text-primary)" : "var(--text-secondary)", border: "none", borderRadius: "4px", cursor: "pointer", boxShadow: activeTab === "marketplace" ? "0 1px 3px rgba(0,0,0,0.1)" : "none" }}
          >
            Marketplace
          </button>
        </div>

        <div style={{ position: "relative" }}>
          <Search size={14} style={{ position: "absolute", left: "8px", top: "8px", color: "var(--text-tertiary)" }} />
          <input
            type="text"
            placeholder="Search by ID or name..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 8px 6px 28px",
              backgroundColor: "var(--bg-tertiary)",
              border: "1px solid var(--border-primary)",
              borderRadius: "4px",
              color: "var(--text-primary)",
              fontSize: "13px",
              outline: "none"
            }}
          />
        </div>
      </div>

      {/* Extension List */}
      <div style={{ flex: 1, overflowY: "auto", padding: "8px" }}>
        {loading ? (
          <div style={{ display: "flex", justifyContent: "center", padding: "20px", color: "var(--text-tertiary)" }}>
            <Loader2 size={24} className="spin" />
          </div>
        ) : activeTab === "installed" ? (
          installedExts.length === 0 ? (
            <div style={{ textAlign: "center", padding: "20px", color: "var(--text-tertiary)", fontSize: "13px" }}>
              No Wasm extensions installed yet.
            </div>
          ) : (
            installedExts
              .filter(e => e.name.toLowerCase().includes(searchQuery.toLowerCase()) || e.id.toLowerCase().includes(searchQuery.toLowerCase()))
              .map((ext) => (
                <div 
                  key={ext.id}
                  onClick={() => onFileSelect?.(`__extension__:${ext.id}`)}
                  style={{ 
                    display: "flex", gap: "10px", padding: "10px", 
                    backgroundColor: "var(--bg-primary)", 
                    border: "1px solid var(--border-primary)",
                    borderRadius: "6px", marginBottom: "8px",
                    cursor: "pointer"
                  }}>
                  {ext.icon ? (
                    <img 
                      src={ext.icon} 
                      alt={ext.name} 
                      style={{ width: "36px", height: "36px", borderRadius: "6px", objectFit: "cover" }} 
                    />
                  ) : (
                    <div style={{ width: "36px", height: "36px", backgroundColor: "#3b82f6", borderRadius: "6px", display: "flex", alignItems: "center", justifyContent: "center" }}>
                      <Box size={20} color="white" />
                    </div>
                  )}
                  <div style={{ flex: 1 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                      <div style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "4px" }}>
                        {ext.name}
                        {ext.publisher?.verified && <ShieldCheck size={14} color="#3b82f6" title="Verified Publisher" />}
                      </div>
                      <button 
                        onClick={(e) => { e.stopPropagation(); onFileSelect?.(`__extension__:${ext.id}`); }}
                        style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-tertiary)", padding: 0 }}
                      >
                        <Settings size={14} />
                      </button>
                    </div>
                    <div style={{ fontSize: "10px", color: "var(--text-tertiary)", fontFamily: "monospace" }}>
                      {ext.id}
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-secondary)", margin: "4px 0", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                      {ext.description || "WASI Sandboxed Plugin"}
                    </div>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "6px" }}>
                      <span style={{ fontSize: "10px", color: "#10b981", display: "flex", alignItems: "center", gap: "3px" }}>
                        <ShieldCheck size={12} /> SHA-256 Verified
                      </span>
                      <span style={{ fontSize: "11px", color: "var(--text-tertiary)" }}>v{ext.version}</span>
                    </div>
                  </div>
                </div>
              ))
          )
        ) : (
          marketplaceExts.length === 0 ? (
            <div style={{ textAlign: "center", padding: "20px", color: "var(--text-tertiary)", fontSize: "13px" }}>
              No Wasm extensions found in registry.
            </div>
          ) : (
            marketplaceExts
              .filter(e => e.name.toLowerCase().includes(searchQuery.toLowerCase()) || e.id.toLowerCase().includes(searchQuery.toLowerCase()))
              .map((item) => (
                <div 
                  key={item.id}
                  onClick={() => onFileSelect?.(`__extension__:${item.id}`)}
                  style={{ 
                    display: "flex", gap: "10px", padding: "10px", 
                    backgroundColor: "var(--bg-primary)", 
                    border: "1px solid var(--border-primary)",
                    borderRadius: "6px", marginBottom: "8px",
                    cursor: "pointer"
                  }}>
                  {item.icon ? (
                    <img 
                      src={item.icon} 
                      alt={item.name} 
                      style={{ width: "36px", height: "36px", borderRadius: "6px", objectFit: "cover" }} 
                    />
                  ) : (
                    <div style={{ width: "36px", height: "36px", backgroundColor: "#8b5cf6", borderRadius: "6px", display: "flex", alignItems: "center", justifyContent: "center" }}>
                      <Box size={20} color="white" />
                    </div>
                  )}
                  <div style={{ flex: 1 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                      <div style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "4px" }}>
                        {item.name}
                        {item.publisher?.verified && <ShieldCheck size={14} color="#3b82f6" title="Verified Publisher" />}
                      </div>
                      <span style={{ fontSize: "11px", color: "var(--text-tertiary)" }}>v{item.version}</span>
                    </div>
                    <div style={{ fontSize: "10px", color: "var(--text-tertiary)", fontFamily: "monospace" }}>
                      {item.id}
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-secondary)", margin: "4px 0" }}>
                      {item.description || "Serverless Wasm extension"}
                    </div>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "8px" }}>
                      <span style={{ fontSize: "10px", color: "var(--text-tertiary)" }}>
                        Perms: {item.permissions.length > 0 ? item.permissions.join(", ") : "Zero-Trust (0)"}
                      </span>
                      <button 
                        onClick={(e) => handleInstall(item, e)}
                        disabled={installingId === item.id}
                        style={{ 
                          background: "#3b82f6", 
                          color: "white", 
                          border: "none", 
                          borderRadius: "4px", 
                          padding: "3px 10px", 
                          fontSize: "11px", 
                          cursor: "pointer",
                          display: "flex",
                          alignItems: "center",
                          gap: "4px"
                        }}>
                        {installingId === item.id ? (
                          <Loader2 size={12} className="spin" />
                        ) : (
                          <Download size={12} />
                        )}
                        Install
                      </button>
                    </div>
                  </div>
                </div>
              ))
          )
        )}
      </div>
    </div>
  );
}
