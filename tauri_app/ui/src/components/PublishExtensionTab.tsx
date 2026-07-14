import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UploadCloud, FolderOpen, Loader2, Info } from "lucide-react";

export default function PublishExtensionTab() {
  const [folderPath, setFolderPath] = useState("");
  const [version, setVersion] = useState("1.0.0");
  const [changelog, setChangelog] = useState("Initial release");
  const [isPublishing, setIsPublishing] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  const handleSelectFolder = async () => {
    try {
      const selected = await invoke<string | null>("open_folder_dialog");
      if (selected && typeof selected === "string") {
        setFolderPath(selected);
        setError("");
        setSuccess("");
      }
    } catch (err) {
      console.error(err);
    }
  };

  const handlePublish = async () => {
    if (!folderPath || !version) {
      setError("Please select a folder and enter a version.");
      return;
    }

    setIsPublishing(true);
    setError("");
    setSuccess("");

    try {
      await invoke("publish_extension", {
        folderPath,
        version,
        changelog,
      });
      setSuccess("Extension published successfully!");
      setFolderPath("");
      setVersion("1.0.0");
      setChangelog("");
    } catch (err: any) {
      console.error(err);
      setError(err.toString());
    } finally {
      setIsPublishing(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)", overflowY: "auto" }}>
      {/* Header section */}
      <div style={{ display: "flex", gap: "20px", padding: "40px", borderBottom: "1px solid var(--border-primary)", backgroundColor: "var(--bg-secondary)" }}>
        <div style={{ width: "80px", height: "80px", backgroundColor: "var(--color-accent)", borderRadius: "12px", display: "flex", alignItems: "center", justifyContent: "center", boxShadow: "0 4px 12px rgba(0,0,0,0.1)" }}>
          <UploadCloud size={40} color="white" />
        </div>
        <div style={{ flex: 1, display: "flex", flexDirection: "column", justifyContent: "center" }}>
          <h1 style={{ margin: "0 0 8px 0", fontSize: "28px", display: "flex", alignItems: "center", gap: "12px" }}>
            Publish New Extension
          </h1>
          <div style={{ fontSize: "14px", color: "var(--text-secondary)" }}>
            Package your local extension and publish it to the Marketplace.
          </div>
        </div>
      </div>

      {/* Body section */}
      <div style={{ padding: "40px", maxWidth: "800px", display: "flex", flexDirection: "column", gap: "24px" }}>
        
        {error && (
          <div style={{ color: "var(--color-danger)", fontSize: "14px", padding: "12px", backgroundColor: "rgba(239, 68, 68, 0.1)", border: "1px solid rgba(239, 68, 68, 0.2)", borderRadius: "6px" }}>
            {error}
          </div>
        )}
        
        {success && (
          <div style={{ color: "var(--color-success)", fontSize: "14px", padding: "12px", backgroundColor: "rgba(34, 197, 94, 0.1)", border: "1px solid rgba(34, 197, 94, 0.2)", borderRadius: "6px" }}>
            {success}
          </div>
        )}

        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)" }}>Extension Folder</label>
          <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "4px", display: "flex", alignItems: "center", gap: "6px" }}>
            <Info size={14} /> Folder must contain a valid `proto-extension.json` file.
          </div>
          <div style={{ display: "flex", gap: "12px" }}>
            <input 
              type="text" 
              value={folderPath} 
              readOnly
              placeholder="Select the folder containing proto-extension.json..."
              style={{ flex: 1, padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
            />
            <button 
              onClick={handleSelectFolder}
              style={{ padding: "0 16px", backgroundColor: "var(--bg-tertiary)", border: "1px solid var(--border-primary)", borderRadius: "6px", cursor: "pointer", color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "8px", fontWeight: 500 }}
            >
              <FolderOpen size={16} /> Browse
            </button>
          </div>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)" }}>Version</label>
          <input 
            type="text" 
            value={version} 
            onChange={e => setVersion(e.target.value)}
            placeholder="e.g. 1.0.0"
            style={{ width: "200px", padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
          />
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)" }}>Changelog / Release Notes</label>
          <textarea 
            value={changelog}
            onChange={e => setChangelog(e.target.value)}
            rows={5}
            placeholder="Describe what's new in this version..."
            style={{ width: "100%", padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", resize: "vertical", fontSize: "14px", fontFamily: "inherit" }}
          />
        </div>

        <div style={{ marginTop: "16px", paddingTop: "24px", borderTop: "1px solid var(--border-primary)", display: "flex" }}>
          <button 
            onClick={handlePublish} 
            disabled={isPublishing || !folderPath}
            style={{ 
              padding: "12px 24px", 
              backgroundColor: folderPath ? "var(--color-accent)" : "var(--bg-tertiary)", 
              color: folderPath ? "white" : "var(--text-tertiary)", 
              border: "none", 
              borderRadius: "6px", 
              cursor: folderPath && !isPublishing ? "pointer" : "not-allowed", 
              display: "flex", 
              alignItems: "center", 
              gap: "8px", 
              fontWeight: 600,
              fontSize: "14px",
              transition: "all 0.2s"
            }}
          >
            {isPublishing ? <Loader2 size={16} className="spin" /> : <UploadCloud size={16} />}
            {isPublishing ? "Publishing to Marketplace..." : "Publish Extension"}
          </button>
        </div>
      </div>
    </div>
  );
}
