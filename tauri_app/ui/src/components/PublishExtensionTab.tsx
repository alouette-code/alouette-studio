import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UploadCloud, FolderOpen, Loader2, ShieldCheck, GitPullRequest, Copy, Check, Image as ImageIcon, Lock } from "lucide-react";

export default function PublishExtensionTab() {
  const [publisherId, setPublisherId] = useState("");
  const [extSlug, setExtSlug] = useState("");
  const [name, setName] = useState("");
  const [version, setVersion] = useState("1.0.0");
  const [description, setDescription] = useState("");
  const [icon, setIcon] = useState("");
  const [wasmUrl, setWasmUrl] = useState("");
  const [readmeUrl, setReadmeUrl] = useState("");
  const [repository, setRepository] = useState("");
  const [sha256, setSha256] = useState("");
  const [permissions, setPermissions] = useState<string[]>(["fs:read", "net:http"]);
  
  const [isPublishing, setIsPublishing] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  const handleSelectWasmFile = async () => {
    try {
      const selected = await invoke<string | null>("open_file_dialog");
      if (selected && typeof selected === "string") {
        const hash = await invoke<string>("calculate_wasm_sha256", { filePath: selected });
        setSha256(hash);
        setError("");
        setSuccess("Auto-calculated SHA-256 Checksum from selected Wasm file!");
      }
    } catch (err: any) {
      setError(`Failed to calculate SHA-256 from local file: ${err}`);
    }
  };

  const handleSelectLocalIcon = async () => {
    try {
      const selected = await invoke<string | null>("open_file_dialog");
      if (selected && typeof selected === "string") {
        const generatedUrl = await invoke<string>("generate_extension_icon_uuid", { filePath: selected });
        setIcon(generatedUrl);
        setError("");
        setSuccess(`Generated unique 36-char UUID icon asset URL! Save image to 'icons/' folder in registry.`);
      }
    } catch (err: any) {
      setError(`Failed to process local icon image: ${err}`);
    }
  };

  const fullId = publisherId && extSlug ? `${publisherId.trim()}.${extSlug.trim()}` : "publisher.extension_name";

  const generatedPayload = JSON.stringify(
    {
      id: fullId,
      name: name || "My Wasm Extension",
      version: version || "1.0.0",
      description: description || undefined,
      publisher: {
        id: publisherId || "publisher",
        name: publisherId || "Publisher Name",
        verified: true,
      },
      icon: icon || undefined,
      readme_url: readmeUrl || undefined,
      repository: repository || (publisherId && extSlug ? `https://github.com/${publisherId}/${extSlug}` : undefined),
      wasm_url: wasmUrl || "https://github.com/user/repo/releases/download/v1.0.0/plugin.wasm",
      sha256: sha256 || "AUTO_CALCULATED_SHA256",
      permissions,
    },
    null,
    2
  );

  const handleCopyPayload = () => {
    navigator.clipboard.writeText(generatedPayload);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleTogglePermission = (perm: string) => {
    if (permissions.includes(perm)) {
      setPermissions(permissions.filter((p) => p !== perm));
    } else {
      setPermissions([...permissions, perm]);
    }
  };

  const handlePublish = async () => {
    if (!wasmUrl) {
      setError("Please provide a GitHub Wasm Direct Download URL.");
      return;
    }

    if (!sha256) {
      setError("Please select your local .wasm binary to auto-calculate the SHA-256 Checksum.");
      return;
    }

    setIsPublishing(true);
    setError("");
    setSuccess("");

    try {
      setSuccess(`Generated Registry Payload for '${fullId}' (v${version})! Ready for PR.`);
    } catch (err: any) {
      setError(err.toString());
    } finally {
      setIsPublishing(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", backgroundColor: "var(--bg-primary)", color: "var(--text-primary)", overflowY: "auto" }}>
      {/* Header section */}
      <div style={{ display: "flex", gap: "20px", padding: "40px", borderBottom: "1px solid var(--border-primary)", backgroundColor: "var(--bg-secondary)" }}>
        <div style={{ width: "80px", height: "80px", backgroundColor: "#8b5cf6", borderRadius: "12px", display: "flex", alignItems: "center", justifyContent: "center", boxShadow: "0 4px 12px rgba(0,0,0,0.1)" }}>
          <UploadCloud size={40} color="white" />
        </div>
        <div style={{ flex: 1, display: "flex", flexDirection: "column", justifyContent: "center" }}>
          <h1 style={{ margin: "0 0 8px 0", fontSize: "28px", display: "flex", alignItems: "center", gap: "12px" }}>
            Submit Extension to Marketplace
          </h1>
          <div style={{ fontSize: "14px", color: "var(--text-secondary)" }}>
            Provide your GitHub Release URL, metadata, auto-calculated SHA-256 checksum, and unique UUID icon asset.
          </div>
        </div>
      </div>

      {/* Body Section */}
      <div style={{ padding: "40px", maxWidth: "850px", display: "flex", flexDirection: "column", gap: "24px" }}>
        
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

        {/* Namespace & Unique ID */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", fontWeight: 600 }}>Publisher ID (Namespace)</label>
            <input 
              type="text" 
              value={publisherId} 
              onChange={(e) => setPublisherId(e.target.value)}
              placeholder="e.g. publisher_name"
              style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
            />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", fontWeight: 600 }}>Extension Slug Name</label>
            <input 
              type="text" 
              value={extSlug} 
              onChange={(e) => setExtSlug(e.target.value)}
              placeholder="e.g. extension_name"
              style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
            />
          </div>
        </div>

        <div style={{ fontSize: "12px", color: "#8b5cf6", fontFamily: "monospace", backgroundColor: "var(--bg-secondary)", padding: "8px 12px", borderRadius: "6px" }}>
          Full Qualified Identifier: <strong>{fullId}</strong>
        </div>

        {/* Display Name & Version */}
        <div style={{ display: "grid", gridTemplateColumns: "3fr 1fr", gap: "16px" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", fontWeight: 600 }}>Extension Display Name</label>
            <input 
              type="text" 
              value={name} 
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. My Wasm Plugin"
              style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
            />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", fontWeight: 600 }}>Version</label>
            <input 
              type="text" 
              value={version} 
              onChange={(e) => setVersion(e.target.value)}
              placeholder="e.g. 1.0.0"
              style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
            />
          </div>
        </div>

        {/* Description */}
        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <label style={{ fontSize: "13px", fontWeight: 600 }}>Short Description</label>
          <input 
            type="text" 
            value={description} 
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Describe what your WASM plugin accomplishes..."
            style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
          />
        </div>

        {/* Local Icon Selection with 36-char UUID */}
        <div style={{ display: "flex", flexDirection: "column", gap: "6px", backgroundColor: "var(--bg-secondary)", padding: "16px", borderRadius: "8px", border: "1px solid var(--border-primary)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <label style={{ fontSize: "13px", fontWeight: 600, display: "flex", alignItems: "center", gap: "6px" }}>
              <ImageIcon size={16} color="#8b5cf6" /> Extension Icon (Auto 36-Char Unique UUID)
            </label>
            <button 
              onClick={handleSelectLocalIcon}
              style={{ background: "#8b5cf6", color: "white", border: "none", borderRadius: "4px", padding: "4px 12px", fontSize: "12px", cursor: "pointer", display: "flex", alignItems: "center", gap: "4px" }}
            >
              <FolderOpen size={14} /> Select Local Image (Max 500x500)
            </button>
          </div>
          <input 
            type="text" 
            value={icon} 
            readOnly
            placeholder="Click button above to select local image. Icon URL will auto-generate..."
            style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "13px", fontFamily: "monospace", opacity: 0.9 }}
          />
        </div>

        {/* URLs */}
        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", fontWeight: 600 }}>GitHub Wasm Release Direct Download URL (wasm_url)</label>
            <input 
              type="text" 
              value={wasmUrl} 
              onChange={(e) => setWasmUrl(e.target.value)}
              placeholder="https://github.com/user/repo/releases/download/v1.0.0/plugin.wasm"
              style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
            />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", fontWeight: 600 }}>README Documentation URL (readme_url)</label>
            <input 
              type="text" 
              value={readmeUrl} 
              onChange={(e) => setReadmeUrl(e.target.value)}
              placeholder="https://raw.githubusercontent.com/user/repo/main/README.md"
              style={{ padding: "10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "14px" }}
            />
          </div>
        </div>

        {/* Binary SHA-256 Checksum - READONLY AUTO-CALCULATED */}
        <div style={{ display: "flex", flexDirection: "column", gap: "6px", backgroundColor: "var(--bg-secondary)", padding: "16px", borderRadius: "8px", border: "1px solid var(--border-primary)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <label style={{ fontSize: "13px", fontWeight: 600, display: "flex", alignItems: "center", gap: "6px" }}>
              <ShieldCheck size={16} color="#10b981" /> Binary SHA-256 Checksum (Auto Generated)
            </label>
            <button 
              onClick={handleSelectWasmFile}
              style={{ background: "#3b82f6", color: "white", border: "none", borderRadius: "4px", padding: "4px 12px", fontSize: "12px", cursor: "pointer", display: "flex", alignItems: "center", gap: "4px" }}
            >
              <FolderOpen size={14} /> Calculate from Local .wasm
            </button>
          </div>
          <div style={{ position: "relative" }}>
            <input 
              type="text" 
              value={sha256} 
              readOnly
              placeholder="Auto-generated SHA-256 checksum (Click 'Calculate from Local .wasm' button)..."
              style={{ width: "100%", padding: "10px 32px 10px 10px", borderRadius: "6px", border: "1px solid var(--border-primary)", backgroundColor: "var(--bg-tertiary)", color: sha256 ? "#10b981" : "var(--text-tertiary)", fontSize: "13px", fontFamily: "monospace", opacity: 0.95 }}
            />
            <Lock size={14} style={{ position: "absolute", right: "10px", top: "12px", color: "var(--text-tertiary)" }} />
          </div>
        </div>

        {/* Permissions Request */}
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "13px", fontWeight: 600 }}>Requested Permissions (Permission Firewall)</label>
          <div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
            {["fs:read", "fs:write", "net:http", "terminal:exec"].map((p) => (
              <label key={p} style={{ display: "flex", alignItems: "center", gap: "6px", fontSize: "13px", cursor: "pointer", backgroundColor: "var(--bg-tertiary)", padding: "6px 12px", borderRadius: "6px", border: "1px solid var(--border-primary)" }}>
                <input 
                  type="checkbox" 
                  checked={permissions.includes(p)} 
                  onChange={() => handleTogglePermission(p)} 
                />
                <code>{p}</code>
              </label>
            ))}
          </div>
        </div>

        {/* Actions */}
        <div style={{ marginTop: "16px", paddingTop: "24px", borderTop: "1px solid var(--border-primary)", display: "flex", gap: "16px", alignItems: "center" }}>
          <button 
            onClick={handleCopyPayload}
            style={{ 
              padding: "12px 20px", 
              backgroundColor: "var(--bg-tertiary)", 
              color: "var(--text-primary)", 
              border: "1px solid var(--border-primary)", 
              borderRadius: "6px", 
              cursor: "pointer", 
              display: "flex", 
              alignItems: "center", 
              gap: "8px", 
              fontWeight: 600,
              fontSize: "14px"
            }}
          >
            {copied ? <Check size={16} color="#10b981" /> : <Copy size={16} />}
            {copied ? "Copied JSON Payload!" : "Copy JSON Payload"}
          </button>

          <button 
            onClick={handlePublish} 
            disabled={isPublishing}
            style={{ 
              padding: "12px 24px", 
              backgroundColor: "#8b5cf6", 
              color: "white", 
              border: "none", 
              borderRadius: "6px", 
              cursor: isPublishing ? "not-allowed" : "pointer", 
              display: "flex", 
              alignItems: "center", 
              gap: "8px", 
              fontWeight: 600,
              fontSize: "14px"
            }}
          >
            {isPublishing ? <Loader2 size={16} className="spin" /> : <GitPullRequest size={16} />}
            {isPublishing ? "Submitting..." : "Submit Registry Entry"}
          </button>
        </div>
      </div>
    </div>
  );
}
