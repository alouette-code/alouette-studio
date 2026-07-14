import { useState, useEffect } from "react";
import { Search, Settings, Box, Loader2, UploadCloud } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface ExtensionManifest {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
}

interface ExtensionPanelProps {
  onFileSelect?: (path: string) => void;
}

export default function ExtensionPanel({ onFileSelect }: ExtensionPanelProps) {
  const [activeTab, setActiveTab] = useState<"installed" | "marketplace">("installed");
  const [searchQuery, setSearchQuery] = useState("");
  const [extensions, setExtensions] = useState<ExtensionManifest[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchExtensions = async () => {
    setLoading(true);
    try {
      const endpoint = activeTab === "installed" ? "get_installed_extensions" : "fetch_marketplace_extensions";
      const data = await invoke<ExtensionManifest[]>(endpoint);
      setExtensions(data);
    } catch (err) {
      console.error("Failed to load extensions:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchExtensions();
  }, [activeTab]);

  return (
    <div className="extension-panel" style={{ display: "flex", flexDirection: "column", height: "100%", backgroundColor: "var(--bg-secondary)" }}>
      {/* Header & Search */}
      <div style={{ padding: "12px", borderBottom: "1px solid var(--border-primary)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "10px" }}>
          <h3 style={{ margin: 0, fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px" }}>
            Extensions
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
            placeholder="Search extensions..."
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
        ) : extensions.length === 0 ? (
          <div style={{ textAlign: "center", padding: "20px", color: "var(--text-tertiary)", fontSize: "13px" }}>
            No extensions installed.
          </div>
        ) : (
          extensions
            .filter(e => e.name.toLowerCase().includes(searchQuery.toLowerCase()))
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
              <div style={{ width: "36px", height: "36px", backgroundColor: "var(--color-accent)", borderRadius: "4px", display: "flex", alignItems: "center", justifyContent: "center" }}>
                <Box size={20} color="white" />
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                  <div style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-primary)" }}>{ext.name}</div>
                  <button 
                    onClick={(e) => { e.stopPropagation(); /* handle settings */ }}
                    style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-tertiary)", padding: 0 }}
                  >
                    <Settings size={14} />
                  </button>
                </div>
                <div style={{ fontSize: "11px", color: "var(--text-secondary)", margin: "4px 0", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {ext.description || "No description"}
                </div>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "6px" }}>
                  <span style={{ fontSize: "11px", color: "var(--text-tertiary)" }}>v{ext.version || "1.0.0"}</span>
                  {activeTab === "marketplace" && (
                    <button style={{ 
                      background: "var(--color-accent)", 
                      color: "white", 
                      border: "none", 
                      borderRadius: "3px", 
                      padding: "2px 8px", 
                      fontSize: "11px", 
                      cursor: "pointer" 
                    }}>
                      Install
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
